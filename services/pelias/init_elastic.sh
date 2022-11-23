#!/bin/bash

set -xe
set -o pipefail

if [ ! -z "$(find /usr/share/elasticsearch/data -type f)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    mkdir -p /usr/share/elasticsearch/data
    tar -xJf "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" -C /usr/share/elasticsearch/data
elif [ ! -z "${ELASTICSEARCH_ARTIFACT_URL}" ]; then
    echo "Downloading and extracting artifact."
    rm -fr /tmp/elasticsearch.download
    mkdir -p /tmp/elasticsearch.download
    wget --tries=100 -O- "${ELASTICSEARCH_ARTIFACT_URL}" | tar -xJ -C /tmp/elasticsearch.download
    mv /tmp/elasticsearch.download/nodes /usr/share/elasticsearch/data/nodes
    rmdir /tmp/elasticsearch.download
else
    echo "No elasticsearch artifact available."
    exit 1
fi
