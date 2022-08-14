#!/bin/bash

set -xe

if [ ! -z "$(ls -A /usr/share/elasticsearch/data)" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /usr/share/elasticsearch/data && cat "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" | pbzip2 -d | tar -x
else
    echo "Downloading and extracting artifact."
    cd /usr/share/elasticsearch/data && wget -qO- "${ELASTICSEARCH_ARTIFACT_URL}" | pbzip2 -d | tar -x
fi

if [ ! -z "$(ls -A /data/placeholder)" ]; then
    echo "Nothing to do, already have placeholder data"
elif [ -f "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /data/placeholder && cat "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" | pbzip2 -d | tar -x
else
    echo "Downloading and extracting artifact."
    cd /data/placeholder && wget -qO- "${PLACEHOLDER_ARTIFACT_URL}" | pbzip2 -d | tar -x
fi

if [ ! -z "$(ls -A /data/interpolation)" ]; then
    echo "Nothing to do, already have interpolation data"
elif [ -f "${INTERPOLATION_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    cd /data/interpolation && cat "${INTERPOLATION_ARTIFACT_SOURCE_PATH}" | pbzip2 -d | tar -x
else
    echo "Downloading and extracting artifact."
    cd /data/interpolation && wget -qO- "${INTERPOLATION_ARTIFACT_URL}" | pbzip2 -d | tar -x
fi

if [ -f "/config/pelias.json" ]; then
    echo "Nothing to do, already have pelias config"
elif [ -f "${PELIAS_CONFIG_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cp "${PELIAS_CONFIG_ARTIFACT_SOURCE_PATH}" /config/pelias.json
else
    echo "Downloading artifact."
    wget -O /config/pelias "${PELIAS_CONFIG_ARTIFACT_URL}"
fi
