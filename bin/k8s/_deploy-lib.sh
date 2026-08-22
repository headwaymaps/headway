#!/bin/bash
# Shared setup for the deploy scripts: argument parsing, the clean-tree check,
# and applying a namespace's rendered configs. Sourced, not run.

NAMESPACE=""
CONFIG_DIR=""

# Reads `<namespace> [--dev]` into NAMESPACE and CONFIG_DIR, and cds to the repo
# root so the paths the callers use resolve.
function deploy_lib_parse_args() {
    local dev=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --dev)
                dev=true
                shift
                ;;
            *)
                if [ -z "$NAMESPACE" ]; then
                    NAMESPACE="$1"
                else
                    echo "Unknown argument: $1" >&2
                    exit 1
                fi
                shift
                ;;
        esac
    done

    if [ -z "$NAMESPACE" ]; then
        echo "Usage: $0 <namespace> [--dev]"
        echo "Examples:"
        echo "  $0 planet"
        echo "  $0 seattle --dev"
        echo "  $0 planet --dev"
        exit 1
    fi

    cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

    if [ "$dev" = true ]; then
        CONFIG_DIR="k8s/configs/${NAMESPACE}-dev"
    else
        CONFIG_DIR="k8s/configs/${NAMESPACE}"
    fi

    if [ ! -d "$CONFIG_DIR" ]; then
        echo "no rendered configs at ${CONFIG_DIR} - run bin/k8s/generate first" >&2
        exit 1
    fi
}

# update-fetch-urls dirties the tree and a trap puts it back, which can't tell
# your edits from its own - so refuse to start from a dirty tree.
function deploy_lib_require_clean_tree() {
    local modified_files
    modified_files="$(git diff --name-only)"
    if [ -n "$modified_files" ]; then
        echo "$modified_files"
        echo "👆 Modified files in local directory. Clean first."
        echo "git co ."
        exit 1
    fi
}

# The committed configs carry a placeholder artifact host; swap the real one in
# just long enough to apply, then put it back.
function deploy_lib_apply() {
    trap 'bin/revert-fetch-urls' EXIT
    bin/update-fetch-urls

    (cd "$CONFIG_DIR" && kubectl apply -f . -n "$NAMESPACE")
}
