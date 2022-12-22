#!/bin/bash

set -xe

export ESC="\$"

envsubst < /app/config.json.template > /app/config.json
envsubst < /styles/style.json.template > /app/styles/style.json

cat /app/config.json

if [ -z "$HEADWAY_PUBLIC_URL" ]; then
    echo "HEADWAY_PUBLIC_URL was not set for tileserver"
    exit 1;
fi

tileserver-gl-light --config /app/config.json -u $HEADWAY_PUBLIC_URL
