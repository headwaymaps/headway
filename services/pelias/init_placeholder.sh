#!/bin/bash

set -xe

mkdir -p /data/placeholder

if [ ! -z "$(ls -A /data/placeholder)" ]; then
    echo "Nothing to do, already have placeholder data"
elif [ -f "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /data/placeholder && cat "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" | xz --decompress --stdout | tar -x
else
    echo "Downloading and extracting artifact."
    cd /data/placeholder && wget -O- "${PLACEHOLDER_ARTIFACT_URL}" | xz --decompress --stdout | tar -x
fi
