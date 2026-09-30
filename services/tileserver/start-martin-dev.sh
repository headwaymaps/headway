#!/bin/bash

set -xe

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MARTIN_CONFIG="$SCRIPT_DIR/martin-dev-config.yaml"
WORKTREE_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
DATA_PATH="${DATA_PATH:-$WORKTREE_ROOT/data}"
export DATA_PATH

PORT="${PORT:-8000}"
cat "$MARTIN_CONFIG"

PRIMARY_WORKTREE="$(git worktree list | head -n 1 | cut -d' ' -f 1)"
MARTIN_SRC="${PRIMARY_WORKTREE}/../martin"
(cd "$MARTIN_SRC" && cargo build)
(cd "$SCRIPT_DIR" && RUST_LOG=debug "$MARTIN_SRC/target/debug/martin" --config "$MARTIN_CONFIG" --listen-addresses "0.0.0.0:${PORT}")
