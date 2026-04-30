#!/bin/bash

set -xe

PORT="${PORT:-8000}"
MARTIN_CONFIG="${MARTIN_CONFIG:-/app/martin-config.yaml}"
cat "$MARTIN_CONFIG"

martin --config "$MARTIN_CONFIG" --listen-addresses "0.0.0.0:${PORT}"
