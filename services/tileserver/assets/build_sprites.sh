#!/bin/bash
set -euo pipefail

if [ $# -ne 2 ]; then
    echo "Usage: $0 <SPRITES_INPUT_DIR> <SPRITES_OUTPUT_DIR>"
    exit 1
fi

SPRITES_INPUT_DIR="$1"
SPRITES_OUTPUT_DIR="$2"
SPRITE_NAME="sprite"

# Create output directory
mkdir -p "$SPRITES_OUTPUT_DIR"

echo "Generating sprites from $SPRITES_INPUT_DIR"

# Generate regular sprite
spreet "$SPRITES_INPUT_DIR" "${SPRITES_OUTPUT_DIR}/${SPRITE_NAME}"

# Generate retina sprite
spreet --retina "$SPRITES_INPUT_DIR" "${SPRITES_OUTPUT_DIR}/${SPRITE_NAME}@2x"

echo "Sprite generation complete"
