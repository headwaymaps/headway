//! Downloads the curated GTFS feeds for a transit zone and repacks them.
//!
//! Fetches each feed the zone names and writes it back out with a
//! `feed_info.txt` carrying a unique `feed_id`.
//!
//! This replaces `download_gtfs_feeds.py`. Beyond the language, the meaningful
//! change is where the feed identity comes from: the old CSV keyed feeds by
//! MobilityDatabase source id, and the atlas has no such thing, so feeds are
//! now keyed by Onestop ID.

use gtfout::feed_config::FeedConfig;
use gtfout::measure;
use gtfout::zone::{download_auth, Zone};
use gtfout::Result;
use transit_zone::feed_id::{feed_id_for, rewrite_feed_info};

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Download the curated GTFS feeds for a transit zone")]
struct Args {
    /// The zone file, as produced by the zone builder.
    ///
    /// A zone whose credentials are filled in needs no --config.
    #[arg(long)]
    zone: PathBuf,

    /// Directory to write the repacked feed zips into.
    #[arg(long)]
    output: PathBuf,

    /// YAML config of credentials, as read by write-gtfs-index.
    ///
    /// The same file: a feed that needed a credential to be measured needs the
    /// same one to be built. This is where a zone's credentials come from when
    /// the zone itself leaves them blank, which is how a committed one looks.
    #[arg(long)]
    config: Option<PathBuf>,
}

/// One feed to fetch.
#[derive(Debug, PartialEq)]
struct FeedToDownload {
    feed_onestop_id: String,
    provider: String,
    url: String,
    auth: Option<measure::Auth>,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    std::fs::create_dir_all(&args.output)?;

    let mut credentials = match &args.config {
        Some(path) => FeedConfig::load(path)?,
        None => FeedConfig::default(),
    };

    let zone = Zone::load(&args.zone)?;
    // A zone's own credentials win over any --config, since they're the ones
    // whoever curated the zone verified against these feeds.
    credentials.feeds.extend(zone_credentials(&zone));

    let unfilled = feeds_missing_credentials(&zone, &credentials);
    if !unfilled.is_empty() {
        return Err(format!(
            "no credential for {}; fill in `authorization.credential` for {} in {}",
            unfilled.join(", "),
            if unfilled.len() == 1 { "it" } else { "them" },
            args.zone.display()
        )
        .into());
    }

    let feeds = from_zone(&zone);

    let client = reqwest::blocking::Client::builder()
        .user_agent("headway-download-feeds")
        .build()?;

    for feed in &feeds {
        let feed_id = feed_id_for(&feed.feed_onestop_id);

        eprintln!(
            "Downloading {} ({}) from {}",
            feed_id, feed.provider, feed.url
        );
        let zip_bytes = measure::fetch_feed(
            &client,
            &feed.url,
            &feed.feed_onestop_id,
            feed.auth.as_ref(),
            &credentials,
        )
        .map_err(|e| {
            format!(
                "could not download GTFS feed for {} ({}): {e}",
                feed.provider, feed.feed_onestop_id
            )
        })?;

        let output_path = args.output.join(format!("{feed_id}.gtfs.zip"));
        repack(&zip_bytes, &feed_id, &output_path)?;
    }

    eprintln!("wrote {} feeds to {}", feeds.len(), args.output.display());
    Ok(())
}

fn from_zone(zone: &Zone) -> Vec<FeedToDownload> {
    zone.feeds
        .iter()
        .map(|feed| FeedToDownload {
            feed_onestop_id: feed.feed_onestop_id.clone(),
            provider: feed.provider.clone(),
            url: feed.url.clone(),
            auth: download_auth(feed),
        })
        .collect()
}

/// The zone's literal credentials, in the table `fetch_feed` looks them up in.
///
/// Blank entries are dropped rather than carried through: an unfilled field is
/// something still to supply, and sending it as a key just earns a 401.
fn zone_credentials(zone: &Zone) -> impl Iterator<Item = (String, String)> + '_ {
    zone.feeds.iter().filter_map(|feed| {
        let credential = feed.authorization.as_ref()?.credential.trim();
        (!credential.is_empty()).then(|| (feed.feed_onestop_id.clone(), credential.to_owned()))
    })
}

/// Feeds the zone says need a credential and has no value for.
///
/// Checked up front rather than left to the fetch: otherwise it surfaces as a
/// download failure several feeds in, and against `--config`, which a build
/// driven by a zone file doesn't have.
fn feeds_missing_credentials<'a>(zone: &'a Zone, credentials: &FeedConfig) -> Vec<&'a str> {
    zone.feeds
        .iter()
        .filter(|feed| {
            feed.authorization.is_some() && credentials.get(&feed.feed_onestop_id).is_none()
        })
        .map(|feed| feed.feed_onestop_id.as_str())
        .collect()
}

/// Rewrites the archive with a `feed_info.txt` that names this feed.
fn repack(zip_bytes: &[u8], feed_id: &str, output_path: &Path) -> Result<()> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let existing_feed_info = archive
        .file_names()
        .find(|name| base_name_is(name, "feed_info.txt"))
        .map(str::to_owned);

    let feed_info = match &existing_feed_info {
        Some(name) => {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut archive.by_name(name)?, &mut contents)?;
            rewrite_feed_info(Some(&contents), feed_id)?
        }
        None => rewrite_feed_info(None, feed_id)?,
    };

    let out = std::fs::File::create(output_path)?;
    let mut writer = zip::ZipWriter::new(out);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        if entry.is_dir() || base_name_is(&name, "feed_info.txt") {
            continue;
        }
        writer.start_file(&name, options)?;
        std::io::copy(&mut entry, &mut writer)?;
    }

    // Written last so it replaces whatever the agency shipped.
    writer.start_file("feed_info.txt", options)?;
    writer.write_all(feed_info.as_bytes())?;
    writer.finish()?;

    Ok(())
}

fn base_name_is(path: &str, name: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|base| base.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    const ZONE_JSON: &str = r#"{
      "version": 1,
      "bounds": { "min_lon": -122.5, "min_lat": 47.3, "max_lon": -122.0, "max_lat": 47.8 },
      "feeds": [
        {
          "feed_onestop_id": "f-c23-kingcountymetro",
          "provider": "King County Metro",
          "url": "https://example.com/kcm.zip"
        },
        {
          "feed_onestop_id": "f-9q8y-sfmta",
          "provider": "SFMTA",
          "url": "https://example.com/sf.zip",
          "authorization": { "type": "query_param", "param_name": "api_key",
                             "credential": "static-token" }
        }
      ]
    }"#;

    fn zone() -> Zone {
        Zone::parse(ZONE_JSON).unwrap()
    }

    fn credentials(zone: &Zone) -> BTreeMap<String, String> {
        zone_credentials(zone).collect()
    }

    #[test]
    fn a_zone_names_the_feeds_and_how_to_authenticate_to_them() {
        let feeds = from_zone(&zone());

        assert_eq!(
            feeds[0],
            FeedToDownload {
                feed_onestop_id: "f-c23-kingcountymetro".to_owned(),
                provider: "King County Metro".to_owned(),
                url: "https://example.com/kcm.zip".to_owned(),
                auth: None,
            }
        );
        assert_eq!(
            feeds[1].auth,
            Some(measure::Auth {
                kind: "query_param".to_owned(),
                param_name: Some("api_key".to_owned()),
            })
        );
    }

    /// The point of the zone format for the build: no second file to pass.
    #[test]
    fn a_zone_carries_its_own_credentials() {
        assert_eq!(
            credentials(&zone()).get("f-9q8y-sfmta"),
            Some(&"static-token".to_owned())
        );
    }

    #[test]
    fn an_unfilled_credential_is_not_a_credential() {
        let mut zone = zone();
        zone.feeds[1].authorization.as_mut().unwrap().credential = "  ".to_owned();

        assert!(credentials(&zone).is_empty());
    }

    #[test]
    fn an_unsupplied_credential_is_caught_before_anything_is_fetched() {
        let mut zone = zone();
        zone.feeds[1].authorization.as_mut().unwrap().credential = String::new();

        let mut credentials = FeedConfig::default();
        credentials.feeds.extend(zone_credentials(&zone));
        assert_eq!(
            feeds_missing_credentials(&zone, &credentials),
            ["f-9q8y-sfmta"]
        );

        // A --config alongside the zone file still counts, which is what keeps
        // a half-migrated zone buildable.
        credentials
            .feeds
            .insert("f-9q8y-sfmta".to_owned(), "from-config".to_owned());
        assert!(feeds_missing_credentials(&zone, &credentials).is_empty());
    }
}
