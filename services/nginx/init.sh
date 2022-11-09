#!/bin/bash

set -xe

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

mkdir -p $(dirname ${FONT_ARTIFACT_DEST_PATH})
mkdir -p $(dirname ${SPRITE_ARTIFACT_DEST_PATH})

if [ -f "${FONT_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${FONT_ARTIFACT_DEST_PATH}"
elif [ -f "${FONT_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying font artifact."
    cp "${FONT_ARTIFACT_SOURCE_PATH}" "${FONT_ARTIFACT_DEST_PATH}"
else
    echo "Downloading font artifact."
    wget -O "${FONT_ARTIFACT_DEST_PATH}" "${FONT_ARTIFACT_URL}"
fi

cd $(dirname ${FONT_ARTIFACT_DEST_PATH}) && tar xvf ${FONT_ARTIFACT_DEST_PATH}

if [ -f "${SPRITE_ARTIFACT_DEST_PATH}" ]; then
    echo "Nothing to do, already have ${SPRITE_ARTIFACT_DEST_PATH}"
elif [ -f "${SPRITE_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying sprite artifact."
    cp "${SPRITE_ARTIFACT_SOURCE_PATH}" "${SPRITE_ARTIFACT_DEST_PATH}"
else
    echo "Downloading sprite artifact."
    wget -O "${SPRITE_ARTIFACT_DEST_PATH}" "${SPRITE_ARTIFACT_URL}"
fi

cd $(dirname ${SPRITE_ARTIFACT_DEST_PATH}) && tar xvf ${SPRITE_ARTIFACT_DEST_PATH}

if [ -z $HEADWAY_SHARED_VOL ]; then
    echo "Expecting HEADWAY_SHARED_VOL to be set."
    exit 1;
fi
"$SCRIPT_DIR/generate_config.sh" > $HEADWAY_SHARED_VOL/headway-config.json
