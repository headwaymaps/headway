#!/usr/bin/env python3

import argparse
import csv
import os
from pathlib import Path
import requests
import shutil
import tempfile
import sys
import zipfile


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


# How long to wait on any single feed download. Some agency servers regenerate
# the zip on demand, so this is generous.
DOWNLOAD_TIMEOUT_SECONDS = 300


# Env-file of API keys mounted as a secret by the dagger build. Dagger sandboxes
# module code, so it can't forward host env vars - the keys have to arrive as a
# file. Falls back to the ambient environment when running outside the build.
GTFS_API_KEYS_PATH = "/run/secrets/gtfs-api-keys"


def api_key_env_var(row):
    """Env var holding the API key for feeds that require one.

    Keyed by mdb_source_id since that's the only stable identifier in the CSV,
    e.g. HEADWAY_GTFS_API_KEY_2455 for AC Transit.
    """
    return "HEADWAY_GTFS_API_KEY_" + row["mdb_source_id"]


def load_api_keys():
    """API keys from the mounted secret, overlaid on the environment."""
    keys = dict(os.environ)
    if not os.path.isfile(GTFS_API_KEYS_PATH):
        return keys

    with open(GTFS_API_KEYS_PATH) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            name, value = line.split("=", 1)
            # Tolerate `export FOO=bar` and quoted values, since this is
            # typically a shell env-file like .bin-env.
            name = name.removeprefix("export ").strip()
            keys[name] = value.strip().strip("'\"")
    return keys


API_KEYS = {}


def authenticated_request_args(row):
    """Query params and headers needed to fetch `urls.direct_download`.

    Returns None when the feed needs a key we don't have, which tells the
    caller to skip this URL rather than fetch an error page.

    `urls.authentication_type` follows the MobilityDatabase convention:
    empty/0 = none, 1 = API key as a query param, 2 = API key as a header.
    Either way `urls.api_key_parameter_name` names the param or header.
    """
    auth_type = (row.get("urls.authentication_type") or "0").strip()
    if auth_type in ("", "0"):
        return {}, {}

    api_key = API_KEYS.get(api_key_env_var(row))
    if not api_key:
        return None

    param_name = row.get("urls.api_key_parameter_name") or "api_key"
    if auth_type == "1":
        return {param_name: api_key}, {}
    elif auth_type == "2":
        return {}, {param_name: api_key}
    else:
        eprint("Unknown urls.authentication_type", auth_type, "- treating as unauthenticated")
        return {}, {}


def candidate_urls(row):
    """The URLs to try, best first.

    `urls.direct_download` is the agency's own endpoint and is authoritative.
    `urls.latest` is the MobilityDatabase's mirror, which lags upstream - it has
    served feeds months past their end_date - so it's only a fallback for when
    the agency URL is dead or needs a key we don't have.
    """
    direct_download = (row.get("urls.direct_download") or "").strip()
    if direct_download:
        auth = authenticated_request_args(row)
        if auth is None:
            eprint(
                "Skipping urls.direct_download for",
                row["provider"],
                "- it needs an API key; set",
                api_key_env_var(row),
                "to use it. Falling back to the (possibly stale) mirror.",
            )
        else:
            params, headers = auth
            yield ("urls.direct_download", direct_download, params, headers)

    latest = (row.get("urls.latest") or "").strip()
    if latest:
        yield ("urls.latest", latest, {}, {})


def download_feed(row, zipfile_path):
    """Fetch a feed into zipfile_path, trying each candidate URL in turn.

    Every failure mode here used to be silent: the response body was written
    straight to a .zip with no status check, so a 404 page became a "zip" that
    only blew up later in unpack_archive with nothing pointing at the culprit.
    """
    errors = []
    for source, url, params, headers in candidate_urls(row):
        eprint("Downloading feed for", row["provider"], "from", source, url)
        try:
            response = requests.get(
                url, params=params, headers=headers, timeout=DOWNLOAD_TIMEOUT_SECONDS
            )
            response.raise_for_status()
        except requests.RequestException as e:
            eprint("  failed:", e)
            errors.append(f"{source} ({url}): {e}")
            continue

        with open(zipfile_path, "wb") as f:
            f.write(response.content)

        # A 200 is not proof of a feed: some hosts serve an HTML error page or a
        # JSON "no such object" body with a success status.
        if not zipfile.is_zipfile(zipfile_path):
            preview = response.content[:200]
            eprint("  failed: response is not a zip archive:", preview)
            errors.append(f"{source} ({url}): not a zip archive")
            continue

        return source

    raise RuntimeError(
        "Could not download GTFS feed for "
        + row["provider"]
        + " (mdb_source_id "
        + row["mdb_source_id"]
        + "). Tried:\n  "
        + "\n  ".join(errors or ["no download URLs in the CSV row"])
    )


def main():
    parser = argparse.ArgumentParser(
        description="Download the input GTFS feeds",
    )
    parser.add_argument("--output", required=True, help="output directory")

    assert not sys.stdin.isatty(), "expecting a filtered MobilityDatabase CSV on stdin"
    args = parser.parse_args()

    eprint("args", args)

    global API_KEYS
    API_KEYS = load_api_keys()

    output_dir = args.output
    Path(output_dir).mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory() as tmpdir:
        for row in csv.DictReader(sys.stdin):
            feed_id = "headway-" + row["mdb_source_id"]

            if row["data_type"] != "gtfs":
                eprint("Skipping row", feed_id, "because it's not a gtfs feed")
                continue

            unzipped_name = feed_id + ".gtfs"
            unzipped_path = tmpdir + "/" + unzipped_name
            zipfile_path = tmpdir + "/" + unzipped_name + ".zip"

            download_feed(row, zipfile_path)

            eprint("Unzipping", zipfile_path, "to", unzipped_path)
            shutil.unpack_archive(zipfile_path, unzipped_path)

            eprint("Rewriting agency ID to ensure it's unique across feeds")
            feed_info_fieldnames = None
            feed_info_row = None

            feed_info_path = unzipped_path + "/feed_info.txt"
            if os.path.isfile(feed_info_path):
                with open(feed_info_path, "r") as feed_info_file:
                    feed_info_reader = csv.DictReader(feed_info_file)
                    feed_info_fieldnames = feed_info_reader.fieldnames

                    for feed_info in feed_info_reader:
                        if feed_info_row is not None:
                            # One thing that's weird is that for aggregate feeds,
                            # it's customary to specify multiple entries in feed_info.txt
                            # Since none of the internal entities within the GTFS
                            # archive reference any of these id's, theres no way to
                            # distinguish which feed an individual entity came
                            # from. And thus, OTP must consider all the feeds as
                            # essentially the same.
                            #
                            # It looks like OTP just grabs the first one:
                            #
                            # https://github.com/opentripplanner/OpenTripPlanner/blob/c9f713c639b48164825471c499ce67f58ebeb15b/src/main/java/org/opentripplanner/graph_builder/module/GtfsFeedId.java#L68
                            #
                            # In any case, I'm going to ignore all but the first entry
                            # to simplify this concern.
                            eprint("ignoring subsequent rows in feed_info.txt")
                        feed_info_row = feed_info

                assert (
                    feed_info_row is not None
                ), "expected at least one row in feed_info.text"
            else:
                eprint("No existing feed_info.txt, so we'll synthesize one")
                feed_info_fieldnames = []
                feed_info_row = { }

            # Replace existing csv
            with open(feed_info_path, "w") as feed_info_file:
                # Some feeds don't have a feed_id (e.g. Whatcom County Transit)

                if not "feed_id" in feed_info_fieldnames:
                    feed_info_fieldnames.insert(0, "feed_id")

                # always overwrite feed_id with something unique
                #
                # OTP uses the feed_id as an identifier for joining
                # GTFS-RT feeds.
                #
                # AFAIK there are no references within the GTFS file
                # to this key, so we shouldn't break any consistency by
                # changing it.
                feed_info_row["feed_id"] = feed_id

                # Synthesize required fields to avoid an OTP error like:
                #    > org.onebusaway.csv_entities.exceptions.MissingRequiredFieldException: missing required field: feed_publisher_name
                # It could be that feed_info.txt is missing that field or perhaps missing the feed_info.txt file altogether
                if not "feed_publisher_name" in feed_info_fieldnames:
                    feed_info_fieldnames.append("feed_publisher_name")
                if not "feed_publisher_name" in feed_info_row:
                    feed_info_row["feed_publisher_name"] = f"Feed Publisher: {feed_id}"

                if not "feed_publisher_url" in feed_info_fieldnames:
                    feed_info_fieldnames.append("feed_publisher_url")
                if not "feed_publisher_url" in feed_info_row:
                    feed_info_row["feed_publisher_url"] = f"https://0.0.0.0/missing/feed_publisher_url/feed-id/{feed_id}"

                if not "feed_lang" in feed_info_fieldnames:
                    feed_info_fieldnames.append("feed_lang")
                if not "feed_lang" in feed_info_row:
                    # we have to guess
                    feed_info_row["feed_lang"] = f"en"

                # feed_id,feed_publisher_name,feed_publisher_url,feed_lang,feed_start_date,feed_end_date,feed_version
                # headway-1007,TMB,https://www.tmb.cat,ca,20230612,20231231,165012062023002

                csv_writer = csv.DictWriter(
                    feed_info_file, fieldnames=feed_info_fieldnames
                )
                csv_writer.writeheader()
                csv_writer.writerow(feed_info_row)

            output_path = output_dir + "/" + feed_id + ".gtfs"
            eprint("writing modified zip to", output_path + ".zip")
            shutil.make_archive(output_path, "zip", unzipped_path)


if __name__ == "__main__":
    main()
