#!/bin/bash

set -xe
set -o pipefail

if [ -f /data/valhalla/tiles.tar ]; then
    echo "Nothing to do, already have artifact."
    exit 0
fi

mkdir -p /data/valhalla/

if [ -f "${VALHALLA_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    xz --decompress --stdout ${VALHALLA_ARTIFACT_SOURCE_PATH} > /data/valhalla/tiles.tar
else
    echo "Downloading artifact."
    wget --tries=100 -O- "${VALHALLA_ARTIFACT_URL}" | xz --decompress --stdout > /data/valhalla/tiles.tar.download
    mv /data/valhalla/tiles.tar.download /data/valhalla/tiles.tar
fi

valhalla_build_config --mjolnir-timezone /data/timezones.sqlite --mjolnir-admin /data/admins.sqlite > /data/valhalla.json

chown -R valhalla /data

ls -lR /data
