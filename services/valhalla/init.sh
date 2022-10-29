#!/bin/bash

set -xe


if [ -f "${VALHALLA_ARTIFACT_LOCK}" ]; then
    echo "Nothing to do, already have artifact."
    exit 0
fi

mkdir -p /data/valhalla/

if [ -f "${VALHALLA_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cd /data && xz --decompress --stdout ${VALHALLA_ARTIFACT_SOURCE_PATH} > /data/valhalla/tiles.tar
else
    echo "Downloading artifact."
    cd /data && wget -O- "${VALHALLA_ARTIFACT_URL}" | xz --decompress --stdout > /data/valhalla/tiles.tar
fi

valhalla_build_config --mjolnir-timezone /data/timezones.sqlite --mjolnir-admin /data/admins.sqlite > /data/valhalla.json

chown -R valhalla /data

touch ${VALHALLA_ARTIFACT_LOCK}

ls -lR /data
