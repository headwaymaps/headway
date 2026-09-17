# Run jobs in parallel, each with a live pane showing its latest output.
#
# Jobs past the concurrency limit wait for a free slot. A pane is recycled when
# its job finishes, and the finished job leaves a single line in the scrollback,
# so this works the same for four long checks or a hundred short uploads.
#
#     source bin/_panes.sh
#     panes_init 4
#     panes_add "frontend" bash -c "yarn lint && yarn test"
#     panes_add "rust" cargo test
#     panes_run || panes_dump_failures
#
# Callers that need their own EXIT trap should call panes_cleanup from it.
# After panes_run, panes_names and panes_statuses are parallel arrays.

panes_names=()
panes_statuses=()

_panes_cmds=()
_panes_logs=()
_panes_dones=()
_panes_pids=()
_panes_reaped=()
_panes_slots=4
_panes_dir=""
_panes_live=""
_panes_height=0

# Set up a run of at most $1 concurrent jobs, or all of them at once if 0.
panes_init() {
    _panes_slots=${1:-4}
    _panes_dir=$(mktemp -d)

    # A terminal gets live panes; a pipe or a CI log gets plain completion lines.
    if [ -t 1 ] && command -v tput > /dev/null 2>&1; then
        _panes_live=1
    else
        _panes_live=""
    fi
}

panes_cleanup() {
    if [ -n "$_panes_live" ]; then
        tput cnorm 2> /dev/null
    fi
    if [ -n "$_panes_dir" ]; then
        rm -rf "$_panes_dir"
    fi
}

# Queue a job: panes_add <name> <command> [args...]
panes_add() {
    local name=$1
    shift
    local i=${#panes_names[@]}
    local cmd
    printf -v cmd '%q ' "$@"

    panes_names+=("$name")
    panes_statuses+=("")
    _panes_cmds+=("$cmd")
    _panes_logs+=("${_panes_dir}/${i}.log")
    _panes_dones+=("${_panes_dir}/${i}.status")
    _panes_pids+=("")
    _panes_reaped+=("")
}

# Run everything queued, rendering until the last job lands.
# Returns non-zero if any job failed.
panes_run() {
    local any_failed=0 pending block

    if [ ${#panes_names[@]} -eq 0 ]; then
        return 0
    fi

    if [ -n "$_panes_live" ]; then
        tput civis 2> /dev/null
    fi

    while :; do
        if [ -n "$_panes_live" ] && [ "$_panes_height" -gt 0 ]; then
            printf '\033[%dA\033[J' "$_panes_height"
            _panes_height=0
        fi

        _panes_reap || any_failed=1
        _panes_fill

        pending=0
        for i in "${!panes_names[@]}"; do
            if [ -z "${_panes_reaped[$i]}" ]; then
                pending=1
            fi
        done

        if [ -n "$_panes_live" ]; then
            block=$(_panes_render)
            if [ -n "$block" ]; then
                printf '%s\n' "$block"
                _panes_height=$(printf '%s\n' "$block" | wc -l)
            fi
        fi

        if [ "$pending" -eq 0 ]; then
            break
        fi
        sleep 0.4
    done

    if [ -n "$_panes_live" ]; then
        tput cnorm 2> /dev/null
    fi
    return $any_failed
}

# Print the full log of every job that failed.
panes_dump_failures() {
    local i
    for i in "${!panes_names[@]}"; do
        if [ "${panes_statuses[$i]}" != "0" ]; then
            echo ""
            echo "===== ${panes_names[$i]} ====="
            cat "${_panes_logs[$i]}"
            echo "===== ❌ ${panes_names[$i]} failed ====="
        fi
    done
}

# Start queued jobs until the concurrency limit is reached.
_panes_fill() {
    local i running=0

    for i in "${!panes_names[@]}"; do
        if [ -n "${_panes_pids[$i]}" ] && [ -z "${_panes_reaped[$i]}" ]; then
            running=$((running + 1))
        fi
    done

    for i in "${!panes_names[@]}"; do
        if [ "$_panes_slots" -gt 0 ] && [ "$running" -ge "$_panes_slots" ]; then
            return 0
        fi
        if [ -z "${_panes_pids[$i]}" ]; then
            : > "${_panes_logs[$i]}"
            (
                eval "${_panes_cmds[$i]}" > "${_panes_logs[$i]}" 2>&1
                echo $? > "${_panes_dones[$i]}"
            ) &
            _panes_pids[$i]=$!
            running=$((running + 1))
        fi
    done
}

# Collect jobs that have landed since the last tick, one scrollback line each.
_panes_reap() {
    local i status ok=0

    for i in "${!panes_names[@]}"; do
        if [ -n "${_panes_reaped[$i]}" ] || [ -z "${_panes_pids[$i]}" ]; then
            continue
        fi
        if [ ! -f "${_panes_dones[$i]}" ]; then
            continue
        fi

        wait "${_panes_pids[$i]}" 2> /dev/null
        status=$(cat "${_panes_dones[$i]}" 2> /dev/null || echo 1)
        panes_statuses[$i]=$status
        _panes_reaped[$i]=1

        if [ "$status" = "0" ]; then
            echo "✅ ${panes_names[$i]}"
        else
            echo "❌ ${panes_names[$i]}"
            ok=1
        fi
    done

    return $ok
}

# The tail of a log, stripped of the carriage returns and color codes that would
# otherwise smear across a fixed-height pane.
_panes_body() {
    tail -c 16384 "$1" 2> /dev/null \
        | tr '\r' '\n' \
        | LC_ALL=C sed $'s/\033\[[0-9;?]*[a-zA-Z]//g' \
        | grep -v '^[[:space:]]*$' \
        | tail -n "$2" \
        | cut -c "1-$3"
}

# A horizontal rule filling the width, captioned with a job name when given.
_panes_rule() {
    local label=$1 cols=$2
    local head fill

    if [ -n "$label" ]; then
        head="── ${label} "
    else
        head="──"
    fi

    printf -v fill '%*s' "$((cols - ${#head}))" ''
    printf '%s%s\n' "$head" "${fill// /─}"
}

# A pane for each running job, sharing whatever vertical room the terminal has.
_panes_render() {
    local i running=0 rows cols pane_lines count line

    for i in "${!panes_names[@]}"; do
        if [ -n "${_panes_pids[$i]}" ] && [ -z "${_panes_reaped[$i]}" ]; then
            running=$((running + 1))
        fi
    done
    if [ "$running" -eq 0 ]; then
        return 0
    fi

    rows=$(tput lines 2> /dev/null || echo 24)
    cols=$(tput cols 2> /dev/null || echo 80)

    # Divide the screen among the running panes, keeping a few rows in hand so
    # the frame never scrolls - the redraw walks back up by its own height.
    # Each pane spends a row on its rule, and one more closes the frame.
    pane_lines=$(((rows - 4) / running - 1))
    if [ "$pane_lines" -lt 1 ]; then
        pane_lines=1
    fi

    for i in "${!panes_names[@]}"; do
        if [ -z "${_panes_pids[$i]}" ] || [ -n "${_panes_reaped[$i]}" ]; then
            continue
        fi

        _panes_rule "${panes_names[$i]}" "$cols"
        count=0
        while IFS= read -r line; do
            echo "   ${line}"
            count=$((count + 1))
        done < <(_panes_body "${_panes_logs[$i]}" "$pane_lines" "$((cols - 3))")
        if [ "$count" -eq 0 ]; then
            echo "   …"
            count=1
        fi
        while [ "$count" -lt "$pane_lines" ]; do
            echo ""
            count=$((count + 1))
        done
    done

    _panes_rule "" "$cols"
}
