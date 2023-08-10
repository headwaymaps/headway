#!/bin/bash

AREA=maps-earth-planet-v1.28

function most_recent() {
    local suffix=$1
    if [[ -z "$suffix" ]]; then
        echo "Must specify suffix for files, e.g. $0 graph.obj.zst"
    fi

    declare -A matches=()
    for graph in data/*.$suffix; do
        if [[ "$graph" =~ data/${AREA}-(.*)-(.*-.*-.*).$suffix ]]; then
            local file_path="${BASH_REMATCH[0]}"
            local content="${BASH_REMATCH[1]}"
            local date="${BASH_REMATCH[2]}"
            # echo "adding matches[$content]=$date"

            # By virtue of ls sorting it's input, the most recent one will
            # clobber any older entry.
            matches[$content]="$file_path"
        fi
    done

    for x in "${!matches[@]}"; do echo "${matches[$x]}" ; done
}

if [[ -z "$1" ]]; then
    echo "Must specify suffix for files, e.g. $0 graph.obj.zst"
fi
most_recent $1
