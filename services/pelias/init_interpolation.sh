#!/bin/bash

set -xe
set -o pipefail

if [ ! -z "$(ls -A /data/interpolation)" ]; then
    echo "Nothing to do, already have interpolation data"
elif [ -f "${INTERPOLATION_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    mkdir -p /data/interpolation
    tar --zstd -xf "${INTERPOLATION_ARTIFACT_SOURCE_PATH}" -C /data/interpolation
elif [ ! -z "${INTERPOLATION_ARTIFACT_URL}" ]; then
    echo "Downloading and extracting artifact."
    rm -fr /tmp/interpolation.download
    mkdir -p /tmp/interpolation.download
    wget --tries=100 -O- "${INTERPOLATION_ARTIFACT_URL}" | tar --zstd -x -C /tmp/interpolation.download
    mv /tmp/interpolation.download /data/interpolation
else
    echo "No interpolation artifact available."
    exit 1
fi
