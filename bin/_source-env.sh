#!/bin/bash
# Wrapper to export variables from build .env for shell scripts
# Usage: source .env.sh <build-dir>
# Example: source .env.sh builds/bogota

CONFIG_DIR="$1"
if [ -z "$CONFIG_DIR" ]; then
    echo "Error: build-config-dir is required" >&2
    echo "Usage: source .env.sh <build-config-dir>" >&2
    echo "Example: source .env.sh builds/bogota" >&2
    return 1
fi

ENV_FILE="${CONFIG_DIR}/.env"

if [ ! -f "$ENV_FILE" ]; then
    echo "Error: $ENV_FILE not found" >&2
    echo "Usage: source .env.sh <build-dir>" >&2
    return 1
fi

set -a
source "$ENV_FILE"
set +a
