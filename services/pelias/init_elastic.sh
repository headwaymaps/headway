#!/bin/bash

set -xe

mkdir -p /usr/share/elasticsearch/data

if [ ! -z "$(find /usr/share/elasticsearch/data -type f)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    tar -xJf "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" -C /usr/share/elasticsearch/data
else
    echo "Downloading and extracting artifact."
    wget -O- "${ELASTICSEARCH_ARTIFACT_URL}" | tar -xJ -C /usr/share/elasticsearch/data
fi
