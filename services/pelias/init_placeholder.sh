#!/bin/bash

set -xe

mkdir -p /data/placeholder

if [ ! -z "$(ls -A /data/placeholder)" ]; then
    echo "Nothing to do, already have placeholder data"
elif [ -f "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    tar -xJf "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" -C /data/placeholder
else
    echo "Downloading and extracting artifact."
    wget -O- "${PLACEHOLDER_ARTIFACT_URL}" | tar -xJ -C /data/placeholder
fi
