#!/bin/bash
set -euo pipefail

# Check for required parameter
if [ $# -ne 1 ]; then
    echo "Usage: $0 <OUTPUT_DIR>"
    exit 1
fi

OUTPUT_DIR="$1"

# Font files and their display names
declare -A FONTS=(
    ["Roboto-Regular.ttf"]="Roboto Regular"
    ["Roboto-Medium.ttf"]="Roboto Medium"
    ["Roboto-Condensed-Italic.ttf"]="Roboto Condensed Italic"
)

mkdir -p "$OUTPUT_DIR"

for font_file in "${!FONTS[@]}"; do
    font_name="${FONTS[$font_file]}"
    echo "Processing font: $font_name"

    mkdir -p "${OUTPUT_DIR}/${font_name}"

    temp_dir="/tmp/$(basename "$font_file" .ttf)"
    mkdir -p "$temp_dir"
    cp "$font_file" "$temp_dir/"

    build_pbf_glyphs "$temp_dir" "${OUTPUT_DIR}/${font_name}"

    rm -rf "$temp_dir"
done

echo "Font generation complete"
