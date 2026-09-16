#!/bin/bash
# Render a millisecond count the way build timings print it.

function fmt_duration() {
    local ms=${1%.*} secs
    secs=$((ms / 1000))
    if [ "$secs" -ge 3600 ]; then
        printf '%dh%02dm' $((secs / 3600)) $(((secs % 3600) / 60))
    elif [ "$secs" -ge 60 ]; then
        printf '%dm%02ds' $((secs / 60)) $((secs % 60))
    elif [ "$secs" -ge 10 ]; then
        printf '%ds' "$secs"
    elif [ "$ms" -ge 1000 ]; then
        printf '%d.%02ds' "$secs" $(((ms % 1000) / 10))
    else
        printf '%dms' "$ms"
    fi
}
