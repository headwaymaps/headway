#!/bin/bash

set -xe

mkdir -p /usr/share/elasticsearch/data

if [ ! -z "$(find /usr/share/elasticsearch/data -type f)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /usr/share/elasticsearch/data && cat "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" | xz --decompress --stdout | tar -x
else
    echo "Downloading and extracting artifact."
    cd /usr/share/elasticsearch/data && wget -O- "${ELASTICSEARCH_ARTIFACT_URL}" | xz --decompress --stdout | tar -x
fi
