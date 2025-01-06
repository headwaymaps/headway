#!/bin/bash

REPO_ROOT=$(git rev-parse --show-toplevel)

# Change to the script's directory
DOWNLOAD_ROOT="${REPO_ROOT}/data/planet-download"
mkdir -p "$DOWNLOAD_ROOT"
cd "$DOWNLOAD_ROOT" || { echo "Failed to change directory to download root."; exit 1; }

# Ensure the environment variable is set
if [ -z "$HEADWAY_PLANET_VERSION" ]; then
  echo "HEADWAY_PLANET_VERSION environment variable is not set."
  exit 1
fi

# Extract the date from the version
PLANET_DATE=$(echo "$HEADWAY_PLANET_VERSION" | grep -oE '[0-9]{6}$')

if [ -z "$PLANET_DATE" ]; then
  echo "Invalid HEADWAY_PLANET_VERSION format. Expected 'v1.YYMMDD'."
  exit 1
fi

# Parse YYMMDD into YYYY/MM/DD
YEAR="20${PLANET_DATE:0:2}"
_MONTH="${PLANET_DATE:2:2}"
_DAY="${PLANET_DATE:4:2}"

# Construct the S3 URLs
BASE_URL="https://osm-planet-us-west-2.s3.amazonaws.com/planet/pbf/${YEAR}"
FILE_NAME="planet-${PLANET_DATE}.osm.pbf"
MD5_NAME="planet-${PLANET_DATE}.osm.pbf.md5"
PBF_URL="${BASE_URL}/${FILE_NAME}"
MD5_URL="${BASE_URL}/${MD5_NAME}"
DESTINATION_PATH="${REPO_ROOT}/maps-earth-planet-v1.${PLANET_DATE}.osm.pbf"

if [ -f "$DESTINATION_PATH" ]; then
  echo "${FILE_NAME} already exists."
  exit 0
fi

# Download the .pbf file
echo "Downloading ${FILE_NAME}..."
if ! aria2c --file-allocation=none --continue=true --max-connection-per-server=16 --split=16 --retry-wait=5 --max-tries=5 "$PBF_URL"
then
  echo "Failed to download ${FILE_NAME}."
  exit 1
fi

# Download the .md5 file
echo "Downloading ${MD5_NAME}..."

if ! aria2c --file-allocation=none --continue=true --max-connection-per-server=16 --split=16 --retry-wait=5 --max-tries=5 "$MD5_URL"
then
  echo "Failed to download ${MD5_NAME}."
  exit 1
fi

# Verify the MD5 checksum
echo "Verifying MD5 checksum..."
if md5sum --check "$MD5_NAME"; then
  echo "MD5 checksum verified successfully."
else
  echo "MD5 checksum verification failed!"
  exit 1
fi

mv "planet-${PLANET_DATE}.osm.pbf" "$DESTINATION_PATH"
