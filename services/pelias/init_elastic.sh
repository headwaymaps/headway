#!/bin/bash

set -xe
set -o pipefail

# Mounted by start_elastic.sh. Pods running alongside each other share it, which
# is safe because nothing writes to it once it's installed.
INDEX_IMAGE=/index/elasticsearch.erofs

function install_image() {
    local partial="${INDEX_IMAGE}.partial"
    zstd -dc > "$partial"
    mv "$partial" "$INDEX_IMAGE"
}

if [ -f "$INDEX_IMAGE" ]; then
    echo "Nothing to do, already have elasticsearch data"
elif [ -f "${ELASTICSEARCH_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Installing existing artifact."
    install_image < "$ELASTICSEARCH_ARTIFACT_SOURCE_PATH"
elif [ -n "${ELASTICSEARCH_ARTIFACT_URL}" ]; then
    echo "Downloading and installing artifact."
    wget --tries=100 -O- "$ELASTICSEARCH_ARTIFACT_URL" | install_image
else
    echo "No elasticsearch artifact available."
    exit 1
fi
