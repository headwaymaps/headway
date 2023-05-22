#!/bin/bash

set -xe
set -o pipefail

if [ -f /data/graph.obj ]; then
    echo "Nothing to do, already have artifact."
elif [ -f "$OTP_ARTIFACT_SOURCE_PATH" ]; then
    echo "Copying artifact."
    zstd --decompress --stdout "$OTP_ARTIFACT_SOURCE_PATH" > /data/graph.obj
elif [ ! -z "$OTP_ARTIFACT_URL" ]; then
    echo "Downloading artifact"
    wget --tries=100 --continue -O- "$OTP_ARTIFACT_URL" | zstd --decompress --stdout > /data/graph.obj.download
    mv /data/graph.obj.download /data/graph.obj
else
    echo "No OTP artifact available."
    exit 1
fi

if [ ! -z "$OTP_ROUTER_CONFIG_JSON" ]; then
    echo "$OTP_ROUTER_CONFIG_JSON" > /data/router-config.json
fi
