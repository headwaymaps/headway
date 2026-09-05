#!/bin/bash

set -xe
set -o pipefail

DATA_DIR=/usr/share/elasticsearch/data

# On the volume itself, so installing is a rename rather than a copy. Unlike
# placeholder we can't do a single atomic rename - the volume is mounted *at*
# DATA_DIR, so there's no parent to rename into. Instead this dir doubles as a
# marker: if it exists at startup, an earlier init died and DATA_DIR is suspect.
EXTRACT_DIR="${DATA_DIR}/.extract"

function extract_elastic() {
    # hardcoded in elasticsearch Dockerfile
    local elasticsearch_group=1000

    rm -fr "$EXTRACT_DIR"
    mkdir "$EXTRACT_DIR"
    tar --zstd -x -f - -C "$EXTRACT_DIR"

    chgrp -R "$elasticsearch_group" "$EXTRACT_DIR"
    chmod -R 'g+rwX' "$EXTRACT_DIR"

    # `*` doesn't match dotfiles, so EXTRACT_DIR survives as a marker until
    # the move completes.
    rm -fr "${DATA_DIR:?}"/*
    mv "${EXTRACT_DIR}"/* "$DATA_DIR"
    rmdir "$EXTRACT_DIR"
}

if [ -e "$EXTRACT_DIR" ]; then
    echo "Found partial data from an interrupted init, discarding it."
    rm -fr "${DATA_DIR:?}"/* "$EXTRACT_DIR"
fi

if [ -n "$(find "$DATA_DIR" -type f)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting existing artifact."
    extract_elastic < "$ELASTICSEARCH_ARTIFACT_SOURCE_PATH"
elif [ -n "${ELASTICSEARCH_ARTIFACT_URL}" ]; then
    echo "Downloading and extracting artifact."
    wget --tries=100 -O- "$ELASTICSEARCH_ARTIFACT_URL" | extract_elastic
else
    echo "No elasticsearch artifact available."
    exit 1
fi
