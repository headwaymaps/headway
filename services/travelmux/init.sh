#!/bin/bash

set -xe
set -o pipefail

ELEVATION_TIFS_DIR="/data/elevation-tifs"

# Check if elevation data already exists
if [ -d "$ELEVATION_TIFS_DIR" ] && [ "$(ls -A $ELEVATION_TIFS_DIR)" ]; then
    echo "Nothing to do, already have elevation data."
    exit 0
fi

# Create elevation tifs directory
mkdir -p "$ELEVATION_TIFS_DIR"

# Download and extract elevation data
if [ -f "$TRAVELMUX_ELEVATION_ARTIFACT_SOURCE_PATH" ]; then
    echo "Copying elevation artifact from local path."
    zstd --decompress --stdout "$TRAVELMUX_ELEVATION_ARTIFACT_SOURCE_PATH" | tar -xf - -C "$ELEVATION_TIFS_DIR"
    ls "$ELEVATION_TIFS_DIR"
elif [ -n "$TRAVELMUX_ELEVATION_ARTIFACT_URL" ]; then
    echo "Downloading elevation artifact from URL: $TRAVELMUX_ELEVATION_ARTIFACT_URL"
    wget --tries=100 --continue -O- "$TRAVELMUX_ELEVATION_ARTIFACT_URL" | zstd --decompress --stdout | tar -xf - -C "$ELEVATION_TIFS_DIR"
else
    echo "No elevation artifact available. Skipping elevation data setup."
    echo "Set TRAVELMUX_ELEVATION_ARTIFACT_SOURCE_PATH or TRAVELMUX_ELEVATION_ARTIFACT_URL to provide elevation data."
    exit 0
fi

# Verify extraction was successful
if [ -d "$ELEVATION_TIFS_DIR" ] && [ "$(ls $ELEVATION_TIFS_DIR)" ]; then
    echo "Successfully extracted elevation data to $ELEVATION_TIFS_DIR"
    ls -la "$ELEVATION_TIFS_DIR"
else
    echo "Warning: No elevation TIF files found after extraction"
fi
