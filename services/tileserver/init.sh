#!/bin/bash

set -xe
set -o pipefail

mkdir -p $(dirname ${MBTILES_ARTIFACT_DEST_PATH})

if [ -f "${MBTILES_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${MBTILES_ARTIFACT_DEST_PATH}"
elif [ -f "${MBTILES_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying mbtiles artifact."
    cp "${MBTILES_ARTIFACT_SOURCE_PATH}" "${MBTILES_ARTIFACT_DEST_PATH}"
elif [ ! -z "${MBTILES_ARTIFACT_URL}" ]; then
    echo "Downloading mbtiles artifact."

    wget --tries=100 --continue -O "${MBTILES_ARTIFACT_DEST_PATH}.download" "${MBTILES_ARTIFACT_URL}"
    WGET_STATUS=$?
    echo "wget exit code was: ${WGET_STATUS}"
    echo "Downloaded mbtiles artifact."
    mv "${MBTILES_ARTIFACT_DEST_PATH}.download" "${MBTILES_ARTIFACT_DEST_PATH}"
else
    echo "No 'area' mbtiles artifact available."
    exit 1
fi

mkdir -p $(dirname ${NATURAL_EARTH_ARTIFACT_DEST_PATH})

if [ -f "${NATURAL_EARTH_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${NATURAL_EARTH_ARTIFACT_DEST_PATH}"
elif [ -f "${NATURAL_EARTH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying natural earth artifact."
    cp "${NATURAL_EARTH_ARTIFACT_SOURCE_PATH}" "${NATURAL_EARTH_ARTIFACT_DEST_PATH}"
elif [ ! -z "${NATURAL_EARTH_ARTIFACT_URL}" ]; then
    echo "Downloading natural earth artifact."
    wget --tries=100 --continue -O "${NATURAL_EARTH_ARTIFACT_DEST_PATH}.download" "${NATURAL_EARTH_ARTIFACT_URL}"
    mv "${NATURAL_EARTH_ARTIFACT_DEST_PATH}.download" "${NATURAL_EARTH_ARTIFACT_DEST_PATH}"
else
    echo "No 'natural earth' mbtiles artifact available."
    exit 1
fi
