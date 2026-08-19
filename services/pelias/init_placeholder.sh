#!/bin/bash

set -xe
set -o pipefail

# Staged on the same filesystem as the destination so the final installation
# is one atomic rename - an interrupted init can't leave a partial
# /data/placeholder that a later run would mistake for complete data.
STAGING_DIR=/data/.placeholder.download

function install_artifact() {
    rm -fr "$STAGING_DIR"
    mkdir -p "$STAGING_DIR"
    tar --zstd -x -f - -C "$STAGING_DIR"
    # If /data/placeholder exists (the image ships an empty one), `mv` would
    # move the staging dir *inside* it rather than into its place.
    rm -fr /data/placeholder
    mv "$STAGING_DIR" /data/placeholder
}

if [ ! -z "$(ls -A /data/placeholder 2>/dev/null)" ]; then
    echo "Nothing to do, already have placeholder data"
elif [ -f "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Extracting artifact."
    install_artifact < "${PLACEHOLDER_ARTIFACT_SOURCE_PATH}"
elif [ ! -z "${PLACEHOLDER_ARTIFACT_URL}" ]; then
    echo "Downloading and extracting artifact."
    wget --tries=100 -O- "${PLACEHOLDER_ARTIFACT_URL}" | install_artifact
else
    echo "No placeholder artifact available."
    exit 1
fi
