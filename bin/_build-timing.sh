#!/bin/bash
# Collect step timings out of a dagger build. Source this, run dagger through
# `timed_dagger`, and print the summary at the end with `build_timing_report`.

# A nested build (bin/build calling bin/build-transit) collects into the outer
# script's file, so its steps land in the one report.
if [ -n "${HEADWAY_TIMING_FILE:-}" ]; then
    TIMING_FILE="$HEADWAY_TIMING_FILE"
    TIMING_OWNER=""
else
    # An explicit template: `mktemp -t <prefix>` means different things to BSD
    # and GNU mktemp, and GNU rejects a template with no X's in it.
    TIMING_FILE=$(mktemp "${TMPDIR:-/tmp}/headway-timings.XXXXXX")
    export HEADWAY_TIMING_FILE="$TIMING_FILE"
    TIMING_OWNER=1
fi

function timing_cleanup() {
    if [ -n "$TIMING_OWNER" ]; then
        rm -f "$TIMING_FILE"
    fi
}

# Run dagger, echoing its output as usual while scraping the HEADWAY_TIMING
# records the module writes into it. The whole invocation is recorded as one
# phase, named by $1. Durations are milliseconds throughout.
function timed_dagger() {
    local phase="$1"
    shift
    local start=$SECONDS
    local status=0

    # The log is scraped afterwards rather than in the pipeline: mawk reads its
    # input a block at a time, so an awk in front of the terminal holds dagger's
    # output back for tens of KiB. tee hands every chunk straight through.
    local log
    log=$(mktemp "${TMPDIR:-/tmp}/headway-dagger-log.XXXXXX")

    set -o pipefail
    dagger --progress=plain "$@" 2>&1 | tee "$log" || status=$?
    set +o pipefail

    scrape_timings "$phase" "$log"
    rm -f "$log"

    printf '%s\t%s\n' "$phase" "$(((SECONDS - start) * 1000))" >> "$TIMING_FILE"
    return $status
}

# Two kinds of record come out of the log: the ones the module writes for each
# artifact it builds, and dagger's own top-level spans - which are sequential,
# so together they account for the whole invocation.
function scrape_timings() {
    awk -v out="$TIMING_FILE" -v phase="$1" '
        # Durations come formatted for people: "0.4s", "3m46s", "1h2m3s".
        function millis(took,   ms, unit, value) {
            ms = 0
            while (match(took, /^[0-9.]+(ms|h|m|s)/)) {
                value = substr(took, RSTART, RLENGTH)
                took = substr(took, RSTART + RLENGTH)
                if (sub(/ms$/, "", value)) { ms += value }
                else if (sub(/h$/, "", value)) { ms += value * 3600000 }
                else if (sub(/m$/, "", value)) { ms += value * 60000 }
                else if (sub(/s$/, "", value)) { ms += value * 1000 }
            }
            return ms
        }
        {
            line = $0
            gsub(/\033\[[0-9;]*m/, "", line)
        }
        line ~ /HEADWAY_TIMING/ {
            sub(/.*HEADWAY_TIMING\t/, "", line)
            print line >> out
            next
        }
        # "<id>  : <name> DONE [1m2.3s]". A nested span is indented with a box
        # drawing character, so a name starting in ASCII is a top-level one.
        line ~ /^[0-9]+ *: *[A-Za-z0-9]/ {
            span = line
            sub(/^[0-9]+ *: */, "", span)
            if (match(span, / (DONE|CACHED) \[[0-9hms.]+\]$/)) {
                took = substr(span, RSTART)
                name = substr(span, 1, RSTART - 1)
                gsub(/[^0-9hms.]/, "", took)
                printf "%s/~%s\t%d\n", phase, name, millis(took) >> out
            }
        }
    ' "$2"
}

function build_timing_report() {
    [ -n "$TIMING_OWNER" ] || return 0
    bin/build-timings --record "$TIMING_FILE" "$@"
}
