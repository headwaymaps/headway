import os
import csv
import sys

if len(sys.argv) != 2:
    print("Error: wrong arguments!")
    print("Usage: %s <mobilitydb.csv>" % sys.argv[0])
    raise "Error: wrong arguments!"

input_path = sys.argv[1]

try:
    bbox = [float(val) for val in os.environ["HEADWAY_BBOX"].strip().split(" ")]
    if len(bbox) != 4:
        raise ValueError("Length != 4")
except:
    print("Invalid or missing environment variable HEADWAY_BBOX")
    raise


def parse_float(str_val):
    # This seems overly permissive.
    try:
        if str_val is None or str_val == "":
            return None
        return float(str_val)
    except ValueError:
        return None


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


with open(input_path) as input_file:
    input_csv = csv.DictReader(input_file)
    with open("/gtfs_feeds/gtfs_feeds.csv", "w") as output_file:
        output_csv = csv.writer(
            output_file, delimiter=",", quotechar='"', quoting=csv.QUOTE_MINIMAL
        )

        for input_row in input_csv:
            if input_row["data_type"] == "gtfs" and gtfs_line_intersects(
                input_row, bbox
            ):
                output_row = [
                    input_row["provider"],
                    input_row["mdb_source_id"],
                    input_row["urls.latest"],
                ]
                output_csv.writerow(output_row)
