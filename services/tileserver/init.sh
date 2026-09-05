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
        # The volume outlives the pod, so a download interrupted by a crash or
        # an eviction leaves a partial .download behind. Discard it and start
        # over rather than resuming onto it: artifact URLs are content
        # addressed, so re-fetching from zero costs bandwidth but can't splice
        # two files together. `wget --continue` used to do exactly that --
        # combined with -O it appends instead of resuming, which is how
        # landcover.mbtiles turned into a malformed SQLite file.
        rm -f "${dest_path}.download"
        wget --tries=100 -O "${dest_path}.download" "$source_path"
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
