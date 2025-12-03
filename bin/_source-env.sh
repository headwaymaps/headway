#!/bin/bash
# Wrapper to export variables from build .env for shell scripts
# Usage: source .env.sh <build-dir>
# Example: source .env.sh builds/Bogota

CONFIG_DIR="$1"
if [ -z "$CONFIG_DIR" ]; then
    echo "Error: build-config-dir is required" >&2
    echo "Usage: source .env.sh <build-config-dir>" >&2
    echo "Example: source $0 builds/Bogota" >&2
    return 1
fi

HEADWAY_ENV_FILE="${HEADWAY_ENV_FILE:-${CONFIG_DIR}/.env}"

if [ ! -f "$HEADWAY_ENV_FILE" ]; then
    echo "Error: $HEADWAY_ENV_FILE not found in $CONFIG_DIR" >&2
    echo "Usage: source $0 <build-dir>" >&2
    return 1
fi

set -a
source "$HEADWAY_ENV_FILE"
set +a
