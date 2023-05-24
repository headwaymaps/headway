#!/usr/bin/env python3

import argparse
import os
import csv
import sys
import fileinput

GTFS_RT_SERVICE_ALERTS = "sa"
GTFS_RT_VEHICLE_POSITIONS = "vp"
GTFS_RT_TRIP_UPDATES = "tu"


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, **kwargs)


def main():
    parser = argparse.ArgumentParser(
        description="Filter the input GTFS feeds to be more relevant",
    )
    parser.add_argument(
        "--bbox",
        required=True,
        help="bounding box to filter input. Specified as <N E S W>",
    )
    parser.add_argument(
        "--gtfs-rt-service-alerts",
        action="store_true",
        help="include any GTFS-RT alert feeds in the output",
    )

    args = parser.parse_args()
    assert not sys.stdin.isatty(), "expecting a MobilityDatabase CSV on stdin"

    try:
        bbox = [float(val) for val in args.bbox.strip().split(" ")]
        if len(bbox) != 4:
            raise ValueError("Length != 4")
    except:
        eprint("Invalid or missing bbox")
        raise

    input_csv = csv.DictReader(sys.stdin)
    output_csv = csv.DictWriter(sys.stdout, fieldnames=input_csv.fieldnames)
    output_csv.writeheader()
    for input_row in input_csv:
        if not gtfs_line_intersects(input_row, bbox):
            continue
        if input_row["data_type"] == "gtfs" or (
            args.gtfs_rt_service_alerts and is_service_alert(input_row)
        ):
            output_csv.writerow(input_row)


def parse_float(str_val):
    # This seems overly permissive.
    try:
        if str_val is None or str_val == "":
            return None
        return float(str_val)
    except ValueError:
        return None


def is_service_alert(input_row):
    return (
        input_row["data_type"] == "gtfs-rt"
        and input_row["entity_type"] == GTFS_RT_SERVICE_ALERTS
    )


def gtfs_line_intersects(row, bbox):
    min_long = parse_float(row["location.bounding_box.minimum_longitude"])
    max_long = parse_float(row["location.bounding_box.maximum_longitude"])
    min_lat = parse_float(row["location.bounding_box.minimum_latitude"])
    max_lat = parse_float(row["location.bounding_box.maximum_latitude"])

    if min_long is None or max_long is None or min_lat is None or max_lat is None:
        return False

    if max_lat - min_lat > 18 or max_long - min_long > 16:
        # This almost certainly just means the transit provider operates "everywhere".
        return False

    # Thers's probably a better way to do this but it's a sunday morning and I haven't had coffee yet.
    if max_long < bbox[0] or max_lat < bbox[1]:
        return False
    if max_long < bbox[0] or min_lat > bbox[3]:
        return False
    if min_long > bbox[2] or max_lat < bbox[1]:
        return False
    if min_long > bbox[2] or min_lat > bbox[3]:
        return False
    return True


if __name__ == "__main__":
    main()
