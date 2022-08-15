#!/bin/bash

set -xe

if [ -f "${ARTIFACT_LOCK}" ]; then
    echo "Nothing to do, already have artifact."
    exit 0
fi

if [ -f "${ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cd / && cat "${ARTIFACT_SOURCE_PATH}" | pbzip2 -d | tar x
else
    echo "Downloading artifact"
    cd / && wget -qO- "${ARTIFACT_URL}" | pbzip2 -d | tar x
fi

chown -R photon /photon

# Don't do this again next time.
touch ${ARTIFACT_LOCK}

ls -lR /photon

# At this point Photon is ready for a warm start. :)
