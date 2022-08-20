#!/bin/bash

set -xe

sleep 5

mkdir -p $(dirname ${MBTILES_ARTIFACT_DEST_PATH})

if [ -f "${MBTILES_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${MBTILES_ARTIFACT_DEST_PATH}"
elif [ -f "${MBTILES_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying sprite artifact."
    cp "${MBTILES_ARTIFACT_SOURCE_PATH}" "${MBTILES_ARTIFACT_DEST_PATH}"
else
    echo "Downloading sprite artifact."
    wget -O "${MBTILES_ARTIFACT_DEST_PATH}" "${MBTILES_ARTIFACT_URL}"
fi
