#!/bin/bash

set -xe

envsubst < /templates/config.json.template > /app/config.json

cat /app/config.json

if [ -z "$HEADWAY_PUBLIC_URL" ]; then
    echo "HEADWAY_PUBLIC_URL was not set for tileserver"
    exit 1;
fi

# The -u option allows the generated configs to reference assets with the
# requisite hostname/path-prefix.
tileserver-gl-light --config /app/config.json -u "${HEADWAY_PUBLIC_URL}/tileserver"
