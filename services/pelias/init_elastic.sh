#!/bin/bash

set -xe
set -o pipefail

DATA_DIR=/usr/share/elasticsearch/data

# The staging dir *is* on the persistent volume, same as the other init
# scripts - so the install below is a rename, not a cross-filesystem copy. What
# we can't do is placeholder's single atomic rename, because the volume is
# mounted *at* elasticsearch's data dir: there's no parent directory to rename
# into. Instead the staging dir doubles as a marker. If it's still here on
# startup, a previous init died partway through and whatever is in DATA_DIR is
# not to be trusted.
EXTRACT_DIR="${DATA_DIR}/.extract"

function extract_elastic() {
    # hardcoded in elasticsearch Dockerfile
    local elasticsearch_group=1000

    rm -fr "$EXTRACT_DIR"
    mkdir "$EXTRACT_DIR"
    tar --zstd -x -f - -C "$EXTRACT_DIR"

    chgrp -R "$elasticsearch_group" "$EXTRACT_DIR"
    chmod -R 'g+rwX' "$EXTRACT_DIR"

    # `*` doesn't match dotfiles, so EXTRACT_DIR survives this and stays a
    # marker right up until the move is complete.
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
