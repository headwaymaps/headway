#!/bin/bash

set -xe

mkdir -p /usr/share/elasticsearch/data

if [ ! -z "$(ls -A /usr/share/elasticsearch/data)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /usr/share/elasticsearch/data && cat "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" | pbzip2 -d | tar -x
else
    echo "Downloading and extracting artifact."
    cd /usr/share/elasticsearch/data && wget -qO- "${ELASTICSEARCH_ARTIFACT_URL}" | pbzip2 -d | tar -x
fi
