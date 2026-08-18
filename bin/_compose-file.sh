#!/bin/bash
# Picks the docker compose file for a build, based on whether that build has
# transit routing enabled.
#
# Source this *after* bin/_source-env.sh - it reads HEADWAY_ENABLE_TRANSIT_ROUTING
# from the build's .env.
#
# Sets COMPOSE_FILE, unless it's already set, so an explicit
# `COMPOSE_FILE=... bin/start-services ...` still wins.

if [ -z "${COMPOSE_FILE:-}" ]; then
    if [ "${HEADWAY_ENABLE_TRANSIT_ROUTING:-0}" = 1 ]; then
        COMPOSE_FILE=docker-compose-with-transit.yaml
    else
        COMPOSE_FILE=docker-compose.yaml
    fi
fi

# Artifact names carry a content hash, so the compose file can't spell them out.
export HEADWAY_OTP_GRAPH_FILE=$(bin/artifacts otp-graph "$CONFIG_DIR")
export HEADWAY_ELEVATION_FILE=$(bin/artifacts elevation "$CONFIG_DIR")
export HEADWAY_PMTILES_FILE=$(bin/artifacts pmtiles "$CONFIG_DIR")
export HEADWAY_VALHALLA_FILE=$(bin/artifacts valhalla "$CONFIG_DIR")
export HEADWAY_ELASTICSEARCH_FILE=$(bin/artifacts elasticsearch "$CONFIG_DIR")
export HEADWAY_PLACEHOLDER_FILE=$(bin/artifacts placeholder "$CONFIG_DIR")
export HEADWAY_TERRAIN_FILE=$(bin/artifacts terrain "$CONFIG_DIR")
export HEADWAY_LANDCOVER_FILE=$(bin/artifacts landcover "$CONFIG_DIR")
