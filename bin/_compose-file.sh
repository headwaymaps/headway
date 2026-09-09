#!/bin/bash

if [ -z "${COMPOSE_FILE:-}" ]; then
    if [ "${HEADWAY_ENABLE_TRANSIT_ROUTING:-0}" = 1 ]; then
        COMPOSE_FILE=docker-compose-with-transit.yaml
    else
        COMPOSE_FILE=docker-compose.yaml
    fi
fi
# Artifact names carry a content hash, so the compose file can't spell them out.
# Allow partial builds, including when Compose is used to stop services.
# One OTP in compose, so the first zone is the one it serves.
if [ "${HEADWAY_ENABLE_TRANSIT_ROUTING:-0}" = 1 ]; then
    export HEADWAY_TRANSIT_ZONE
    HEADWAY_TRANSIT_ZONE=$(bin/artifacts --optional otp-zones "$CONFIG_DIR" | head -1)
    # No zone yet on a partial build, or when Compose is only stopping services.
    if [ -n "$HEADWAY_TRANSIT_ZONE" ]; then
        source bin/_zone-file.sh
        ZONE_FILE=$(zone_file_for "$CONFIG_DIR" "$HEADWAY_TRANSIT_ZONE")
        if [ -z "$ZONE_FILE" ]; then
            echo "Error: no zone.json for ${HEADWAY_TRANSIT_ZONE}" >&2
            exit 1
        fi
        export OTP_ROUTER_CONFIG_JSON
        OTP_ROUTER_CONFIG_JSON=$(cargo run --release --quiet --package transit-zone \
            --bin zone-router-config -- --zone "$ZONE_FILE")
    fi
fi
export HEADWAY_OTP_GRAPH_FILE=$(bin/artifacts --optional otp-graphs "$CONFIG_DIR" | head -1)
export HEADWAY_ELEVATION_FILE=$(bin/artifacts --optional elevation "$CONFIG_DIR")
export HEADWAY_PMTILES_FILE=$(bin/artifacts --optional pmtiles "$CONFIG_DIR")
export HEADWAY_VALHALLA_FILE=$(bin/artifacts --optional valhalla "$CONFIG_DIR")
export HEADWAY_ELASTICSEARCH_FILE=$(bin/artifacts --optional elasticsearch "$CONFIG_DIR")
export HEADWAY_PLACEHOLDER_FILE=$(bin/artifacts --optional placeholder "$CONFIG_DIR")
export HEADWAY_TERRAIN_FILE=$(bin/artifacts --optional terrain "$CONFIG_DIR")
export HEADWAY_LANDCOVER_FILE=$(bin/artifacts --optional landcover "$CONFIG_DIR")
