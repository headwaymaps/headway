#!/bin/bash

set -xe
set -o pipefail

DATA_DIR=/usr/share/elasticsearch/data
# Staged on the persistent volume: elasticsearch expects its data at the
# volume root, so we can't install with a single atomic rename like
# placeholder does. Instead, a leftover extract dir marks an init that died
# part way through installing.
EXTRACT_DIR="${DATA_DIR}/.extract"

function extract_elastic() {
    # hardcoded in elasticsearch Dockerfile
    local elasticsearch_group=1000

    rm -fr "$EXTRACT_DIR"
    mkdir "$EXTRACT_DIR"
    tar --zstd -x -f - -C "$EXTRACT_DIR"

    chgrp -R "$elasticsearch_group" "$EXTRACT_DIR"
    chmod -R 'g+rwX' "$EXTRACT_DIR"

    rm -fr "${DATA_DIR:?}"/*
    mv "${EXTRACT_DIR}"/* "$DATA_DIR"
    rmdir "$EXTRACT_DIR"
}

if [ -e "$EXTRACT_DIR" ]; then
    echo "Found partial data from an interrupted init, discarding it."
    rm -fr "${DATA_DIR:?}"/* "$EXTRACT_DIR"
fi

if [ ! -z "$(find "$DATA_DIR" -type f)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting existing artifact."
    cat "$ELASTICSEARCH_ARTIFACT_SOURCE_PATH" | extract_elastic
elif [ ! -z "${ELASTICSEARCH_ARTIFACT_URL}" ]; then
    echo "Downloading and extracting artifact."
    wget --tries=100 -O- "$ELASTICSEARCH_ARTIFACT_URL" | extract_elastic
else
    echo "No elasticsearch artifact available."
    exit 1
fi
