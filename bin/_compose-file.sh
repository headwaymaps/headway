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

export HEADWAY_OTP_GRAPH_FILE=$(bin/artifacts otp-graph "$CONFIG_DIR")
export HEADWAY_ELEVATION_FILE=$(bin/artifacts elevation "$CONFIG_DIR")
