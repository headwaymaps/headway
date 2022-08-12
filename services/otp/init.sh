#!/bin/bash

set -xe

if [ -f "${ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${ARTIFACT_DEST_PATH}"
    exit 0
fi

if [ -f "${ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cp "${ARTIFACT_SOURCE_PATH}" "${ARTIFACT_DEST_PATH}"
    exit 0
fi

echo "Downloading artifact"
wget -O "${ARTIFACT_DEST_PATH}" "${ARTIFACT_URL}"
