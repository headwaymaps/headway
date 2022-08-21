#!/bin/bash

set -xe

sleep 5

mkdir -p $(dirname ${MBTILES_ARTIFACT_DEST_PATH})

if [ -f "${MBTILES_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${MBTILES_ARTIFACT_DEST_PATH}"
elif [ -f "${MBTILES_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying mbtiles artifact."
    cp "${MBTILES_ARTIFACT_SOURCE_PATH}" "${MBTILES_ARTIFACT_DEST_PATH}"
else
    echo "Downloading mbtiles artifact."
    wget -O "${MBTILES_ARTIFACT_DEST_PATH}" "${MBTILES_ARTIFACT_URL}"
fi

mkdir -p $(dirname ${NATURAL_EARTH_ARTIFACT_DEST_PATH})

if [ -f "${NATURAL_EARTH_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${NATURAL_EARTH_ARTIFACT_DEST_PATH}"
elif [ -f "${NATURAL_EARTH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying natural earth artifact."
    cp "${NATURAL_EARTH_ARTIFACT_SOURCE_PATH}" "${NATURAL_EARTH_ARTIFACT_DEST_PATH}"
else
    echo "Downloading natural earth artifact."
    wget -O "${NATURAL_EARTH_ARTIFACT_DEST_PATH}" "${NATURAL_EARTH_ARTIFACT_URL}"
fi
