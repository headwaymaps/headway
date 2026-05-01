#!/bin/bash

set -xe

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MARTIN_CONFIG="$SCRIPT_DIR/martin-dev-config.yaml"

PORT="${PORT:-8000}"
cat "$MARTIN_CONFIG"

MARTIN_SRC="$SCRIPT_DIR/../../../martin"
(cd $MARTIN_SRC && cargo build)
(cd $SCRIPT_DIR && $MARTIN_SRC/target/debug/martin --config "$MARTIN_CONFIG" --listen-addresses "0.0.0.0:${PORT}")
