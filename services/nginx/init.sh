#!/bin/bash

set -xe

if [ -f "${HEADWAY_BBOX_PATH}" ]; then
    echo "Nothing to do, already have ${HEADWAY_BBOX_PATH}"
    exit 0
fi

if [ -f "${BBOX_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cp "${BBOX_SOURCE_PATH}" "${HEADWAY_BBOX_PATH}"
    exit 0
fi

echo "Downloading artifact"
wget -O "${ARTIFACT_DEST_PATH}" "${ARTIFACT_URL}"
