#!/bin/bash

# Check if any files provided
if [[ $# -eq 0 ]]; then
    echo "Usage: $0 file1.tif [file2.tif ...]" >&2
    exit 1
fi

OUTPUT_SIZE="10%"

# Process each file
for input_file in "$@"; do
    if [[ ! -f "$input_file" ]]; then
        echo "Skipping $input_file (not found)" >&2
        continue
    fi

    # Generate output filename
    base_name="${input_file%.*}"
    output_file="${base_name}_small.tif"

    echo "Processing $input_file..."

    if gdal_translate -q \
        -outsize ${OUTPUT_SIZE} ${OUTPUT_SIZE} \
        -r average \
        -co COMPRESS=LZW \
        "$input_file" "$output_file"; then
        echo "Created $output_file"
    else
        echo "Failed to process $input_file" >&2
    fi
done
