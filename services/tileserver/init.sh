#!/bin/bash

set -xe
set -o pipefail

function download() {
    local source_path=$1
    local dest_path=$2

    mkdir -p "$(dirname "$dest_path")"

    if [[ -f "$dest_path" ]]; then
        echo "Already have ${dest_path}."
    elif [[ $source_path == http* ]]; then
        echo "Downloading ${source_path}..."
        wget --tries=100 --continue -O "${dest_path}.download" "$source_path"
        local wget_status=$?
        echo "wget exit code was: ${wget_status}"
        mv "${dest_path}.download" "$dest_path"
    elif [[ -n "$source_path" ]]; then
        echo "Copying ${source_path}..."
        cp "$source_path" "$dest_path"
    else
        echo "No source specified for ${dest_path}"
        exit 1
    fi
    echo "done"
}

download "$AREAMAP_ARTIFACT_SOURCE" "$AREAMAP_ARTIFACT_DEST"
download "$TERRAIN_ARTIFACT_SOURCE" "$TERRAIN_ARTIFACT_DEST"
download "$LANDCOVER_ARTIFACT_SOURCE" "$LANDCOVER_ARTIFACT_DEST"
