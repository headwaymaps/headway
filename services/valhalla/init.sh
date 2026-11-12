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
    zstd --decompress --stdout ${VALHALLA_ARTIFACT_SOURCE_PATH} > /data/valhalla/tiles.tar
elif [ ! -z "${VALHALLA_ARTIFACT_URL}" ]; then
    echo "Downloading artifact."
    wget --tries=100 -O- "${VALHALLA_ARTIFACT_URL}" | zstd --decompress --stdout > /data/valhalla/tiles.tar.download
    mv /data/valhalla/tiles.tar.download /data/valhalla/tiles.tar
else
    echo "No valhalla artifact available."
    exit 1
fi

# travelmux learns what the tiles cover from the verbose status.
valhalla_build_config --mjolnir-timezone /data/timezones.sqlite --mjolnir-admin /data/admins.sqlite --service-limits-status-allow-verbose True > /data/valhalla.json

chown -R valhalla /data

ls -lR /data
