#!/bin/bash

set -xe
set -o pipefail

if [ -f /data/graph.obj ]; then
    echo "Nothing to do, already have artifact."
elif [ -f "$OTP_ARTIFACT_SOURCE_PATH" ]; then
    echo "Copying artifact."
    cat "$OTP_ARTIFACT_SOURCE_PATH" | zstd --decompress --stdout > /data/graph.obj
elif [ -n "$OTP_ARTIFACT_URL" ]; then
    echo "Downloading artifact"
    wget --tries=100 -O- "$OTP_ARTIFACT_URL" | zstd --decompress --stdout > /data/graph.obj.download
    mv /data/graph.obj.download /data/graph.obj
else
    echo "No OTP artifact available."
    exit 1
fi

# router-config.json is rendered here rather than handed down ready-made,
# because it carries the live GTFS-RT credentials out of gtfs-secrets.json and
# a deployment manifest is a committed file.
ZONE_FILE="${OTP_ZONE_PATH:-/run/config/zone.json}"
SECRETS_FILE="${OTP_GTFS_SECRETS_PATH:-/run/secrets/gtfs-secrets.json}"
TEMP_FILES=()
trap 'rm -f "${TEMP_FILES[@]}"' EXIT

# Tracing off from here on: the secrets and the rendered config both pass
# through this block, and these logs are not private.
set +x

# The cluster mounts the zone and its secrets as files; compose, which has no
# such mechanism, passes the same JSON down in the environment.
if [ -n "${OTP_ZONE_JSON:-}" ]; then
    ZONE_FILE=$(mktemp)
    TEMP_FILES+=("$ZONE_FILE")
    printf '%s\n' "$OTP_ZONE_JSON" > "$ZONE_FILE"
fi
if [ -n "${OTP_GTFS_SECRETS_JSON:-}" ]; then
    SECRETS_FILE=$(mktemp)
    TEMP_FILES+=("$SECRETS_FILE")
    printf '%s\n' "$OTP_GTFS_SECRETS_JSON" > "$SECRETS_FILE"
fi

if [ ! -f "$ZONE_FILE" ]; then
    set -x
    echo "No zone to route: mount one at ${ZONE_FILE} or set OTP_ZONE_JSON" >&2
    exit 1
fi

zone-router-config --zone "$ZONE_FILE" --credentials-file "$SECRETS_FILE" \
    > /data/router-config.json.rendering
mv /data/router-config.json.rendering /data/router-config.json
set -x
echo "Rendered /data/router-config.json"
