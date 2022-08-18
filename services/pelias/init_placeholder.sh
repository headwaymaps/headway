#!/bin/bash

set -xe

mkdir -p /data/placeholder

if [ ! -z "$(ls -A /data/placeholder)" ]; then
    echo "Nothing to do, already have placeholder data"
elif [ -f "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /data/placeholder && cat "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" | pbzip2 -d | tar -x
else
    echo "Downloading and extracting artifact."
    cd /data/placeholder && wget -qO- "${PLACEHOLDER_ARTIFACT_URL}" | pbzip2 -d | tar -x
fi
