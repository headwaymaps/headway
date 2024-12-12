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
MONTH="${PLANET_DATE:2:2}"
DAY="${PLANET_DATE:4:2}"

# Construct the S3 URLs
BASE_URL="https://osm-planet-us-west-2.s3.amazonaws.com/planet/pbf/${YEAR}"
FILE_NAME="planet-${PLANET_DATE}.osm.pbf"
MD5_NAME="planet-${PLANET_DATE}.osm.pbf.md5"
PBF_URL="${BASE_URL}/${FILE_NAME}"
MD5_URL="${BASE_URL}/${MD5_NAME}"
DESINATION_PATH="${REPO_ROOT}/${FILE_NAME}"

if [ -f $DESTINATION_PATH ]; then
    echo "${FILE_NAME} already exists."
    exit 0
fi

# Download the .pbf file
echo "Downloading ${FILE_NAME}..."
aria2c --file-allocation=none --continue=true --max-connection-per-server=16 --split=16 --retry-wait=5 --max-tries=5 "$PBF_URL"

if [ $? -ne 0 ]; then
  echo "Failed to download ${FILE_NAME}."
  exit 1
fi

# Download the .md5 file
echo "Downloading ${MD5_NAME}..."
aria2c --file-allocation=none --continue=true --max-connection-per-server=16 --split=16 --retry-wait=5 --max-tries=5 "$MD5_URL"

if [ $? -ne 0 ]; then
  echo "Failed to download ${MD5_NAME}."
  exit 1
fi

# Verify the MD5 checksum
echo "Verifying MD5 checksum..."
EXPECTED_MD5=$(cat "$MD5_NAME" | awk '{print $1}')
ACTUAL_MD5=$(md5sum "$FILE_NAME" | awk '{print $1}')

if [ "$EXPECTED_MD5" == "$ACTUAL_MD5" ]; then
  echo "MD5 checksum verified successfully."
else
  echo "MD5 checksum verification failed!"
  echo "Expected: $EXPECTED_MD5"
  echo "Actual:   $ACTUAL_MD5"
  exit 1
fi

mv "planet-${PLANET_DATE}.osm.pbf" "$DESINATION_PATH"
