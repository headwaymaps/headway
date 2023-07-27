#!/bin/bash

set -xe
set -o pipefail

function extract_elastic() {
    # hardcoded in elasticsearch Dockerfile
    local elasticsearch_group=1000

    local extract_dir=/tmp/elasticsearch.extract

    rm -fr /tmp/elasticsearch.extract
    mkdir "$extract_dir"
    tar --zstd -x -f - -C "$extract_dir"

    chgrp -R "$elasticsearch_group" "$extract_dir"
    chmod -R 'g+rwX' "$extract_dir"

    rm -fr /usr/share/elasticsearch/data/*
    mv "${extract_dir}"/* /usr/share/elasticsearch/data
}

if [ ! -z "$(find /usr/share/elasticsearch/data -type f)" ]; then
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
