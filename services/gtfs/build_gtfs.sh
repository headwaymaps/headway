#!/bin/bash

set -e
set -o pipefail

# Parse command line arguments
INPUT_DIR=""
OUTPUT_DIR=""

show_help() {
  echo "Usage: $0 --input input_dir --output output_dir"
  echo "  --input input_dir   Directory containing GTFS zip files"
  echo "  --output output_dir  Directory to write processed GTFS files"
  exit 1
}

# Parse long options manually
while [[ $# -gt 0 ]]; do
  case $1 in
    --input)
      INPUT_DIR="$2"
      shift 2
      ;;
    --output)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    *)
      echo "Invalid option: $1" 1>&2
      show_help
      ;;
  esac
done

# Check if required arguments are provided
if [ -z "$INPUT_DIR" ] || [ -z "$OUTPUT_DIR" ]; then
  echo "Error: Both --input and --output arguments are required" 1>&2
  show_help
fi

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR=$(realpath "$OUTPUT_DIR")

mkdir -p unzipped
(cd "$INPUT_DIR" && ls *.zip) | while read zip_file
do
  unzip -d unzipped/$(basename $zip_file .zip) "$INPUT_DIR/$zip_file"
done


for gtfs in unzipped/*
do
  assume-bikes-allowed < "${gtfs}/routes.txt" > tmp-routes.txt
  mv tmp-routes.txt "${gtfs}/routes.txt"
  (cd "$gtfs" && zip -r "${OUTPUT_DIR}/$(basename ${gtfs}).zip" .)
done
