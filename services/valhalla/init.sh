#!/bin/bash

set -xe


if [ -f "${ARTIFACT_LOCK}" ]; then
    echo "Nothing to do, already have artifact."
    exit 0
fi

mkdir -p /data/valhalla/

if [ -f "${ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cd /data && cp ${ARTIFACT_SOURCE_PATH} /data/valhalla/tiles.tar
else
    echo "Downloading artifact."
    cd /data && wget -O /data/valhalla/tiles.tar "${ARTIFACT_URL}"
fi

valhalla_build_config --mjolnir-timezone /data/timezones.sqlite --mjolnir-admin /data/admins.sqlite > /data/valhalla.json

chown -R valhalla /data

touch ${ARTIFACT_LOCK}

ls -lR /data
