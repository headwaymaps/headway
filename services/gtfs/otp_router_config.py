#!/usr/bin/env python3

import argparse
import csv
import sys
import json
from filter_feeds import GTFS_RT_SERVICE_ALERTS, GTFS_RT_TRIP_UPDATES, GTFS_RT_VEHICLE_POSITIONS


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


def main():
    parser = argparse.ArgumentParser(
        description="Output the OTP config for the updaters specified in the input",
    )
    args = parser.parse_args()
    eprint("args", args)

    assert not sys.stdin.isatty(), "expecting a filtered MobilityDatabase CSV on stdin"

    updaters = []
    realtime_feeds = [
        feed for feed in csv.DictReader(sys.stdin) if feed["data_type"] == "gtfs-rt"
    ]
    for feed in realtime_feeds:
        updater_type = None
        frequency_sec = None

        if feed["entity_type"] == GTFS_RT_SERVICE_ALERTS:
            updater_type = "real-time-alerts"
            frequency_sec = 300
        elif feed["entity_type"] == GTFS_RT_TRIP_UPDATES:
            updater_type = "stop-time-updater"
            frequency_sec = 60
        elif feed["entity_type"] == GTFS_RT_VEHICLE_POSITIONS:
            updater_type = "vehicle-positions"
            frequency_sec = 60
        else:
            assert(False, "unknown GTFS-RT updater type", updater_type)

        assert(updater_type != None);
        assert(frequency_sec != None);

        updater = {
            "feedId": "headway-" + feed["static_reference"],
            "type": updater_type,
            # I've chosen some pretty arbitrary defaults. You can always edit them after generating your config.
            "frequencySec": frequency_sec,
            "url": feed["urls.direct_download"],
        }
        updaters.append(updater)
    eprint("updaters", updaters)

    formatted_json = json.dumps({"updaters": updaters}, indent=2)
    print(formatted_json)


if __name__ == "__main__":
    main()
