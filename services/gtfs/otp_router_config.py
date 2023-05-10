#!/usr/bin/env python3

import argparse
import csv
import sys
import json
from filter_feeds import GTFS_RT_SERVICE_ALERTS


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
        assert (
            feed["entity_type"] == GTFS_RT_SERVICE_ALERTS
        ), "only service alerts are currenty supported"
        updater = {
            "feedId": "headway-" + feed["static_reference"],
            "type": "real-time-alerts",
            "frequencySec": 30,
            "url": feed["urls.direct_download"],
        }
        updaters.append(updater)
    eprint("updaters", updaters)

    formatted_json = json.dumps({"updaters": updaters}, indent=2)
    print(formatted_json)


if __name__ == "__main__":
    main()
