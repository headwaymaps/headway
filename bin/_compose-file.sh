#!/bin/bash

source bin/_transit-zones.sh

if [ -z "${COMPOSE_FILE:-}" ]; then
    if [ "$HEADWAY_ENABLE_TRANSIT_ROUTING" = 1 ]; then
        COMPOSE_FILE=docker-compose-with-transit.yaml
    else
        COMPOSE_FILE=docker-compose.yaml
    fi
fi

# Compose runs a single OTP, so the first zone is the one it serves. No zone
# means no transit, which is also how a partial build and `compose down` look.
export HEADWAY_TRANSIT_ZONE="${HEADWAY_TRANSIT_ZONES%%$'\n'*}"
if [ -n "$HEADWAY_TRANSIT_ZONE" ]; then
    source bin/_zone-file.sh
    ZONE_FILE=$(zone_file_for "$CONFIG_DIR" "$HEADWAY_TRANSIT_ZONE")
    if [ -z "$ZONE_FILE" ]; then
        echo "Error: no zone.json for ${HEADWAY_TRANSIT_ZONE}" >&2
        exit 1
    fi
    export OTP_ZONE_JSON OTP_GTFS_SECRETS_JSON
    OTP_ZONE_JSON=$(cat "$ZONE_FILE")
    ZONE_SECRETS_FILE="$(dirname "$ZONE_FILE")/gtfs-secrets.json"
    OTP_GTFS_SECRETS_JSON=""
    if [ -f "$ZONE_SECRETS_FILE" ]; then
        OTP_GTFS_SECRETS_JSON=$(cat "$ZONE_SECRETS_FILE")
    fi
fi

# Artifact names carry a content hash, so the compose file can't spell them out.
# Optional so a partial build, and stopping services, still work.
export HEADWAY_OTP_GRAPH_FILE=$(bin/artifacts --optional otp-graphs "$CONFIG_DIR" | head -1)
export HEADWAY_ELEVATION_FILE=$(bin/artifacts --optional elevation "$CONFIG_DIR")
export HEADWAY_PMTILES_FILE=$(bin/artifacts --optional pmtiles "$CONFIG_DIR")
export HEADWAY_VALHALLA_FILE=$(bin/artifacts --optional valhalla "$CONFIG_DIR")
export HEADWAY_ELASTICSEARCH_FILE=$(bin/artifacts --optional elasticsearch "$CONFIG_DIR")
export HEADWAY_PLACEHOLDER_FILE=$(bin/artifacts --optional placeholder "$CONFIG_DIR")
export HEADWAY_TERRAIN_FILE=$(bin/artifacts --optional terrain "$CONFIG_DIR")
export HEADWAY_LANDCOVER_FILE=$(bin/artifacts --optional landcover "$CONFIG_DIR")
