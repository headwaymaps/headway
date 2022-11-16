#!/bin/bash

set -xe
set -o pipefail

if [ ! -z "$(ls -A /data/placeholder)" ]; then
    echo "Nothing to do, already have placeholder data"
elif [ -f "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    mkdir -p /data/placeholder
    tar -xJf "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" -C /data/placeholder
else
    echo "Downloading and extracting artifact."
    rm -fr /tmp/placeholder.download
    mkdir -p /tmp/placeholder.download
    wget --tries=100 -O- "${PLACEHOLDER_ARTIFACT_URL}" | tar -xJ -C /tmp/placeholder.download
    mv /tmp/placeholder.download /data/placeholder
fi
