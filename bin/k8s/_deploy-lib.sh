#!/bin/bash
# Shared setup for the deploy scripts. Sourced, not run.

set -o pipefail

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

    local config_name="$NAMESPACE"
    if [ "$dev" = true ]; then
        config_name="${NAMESPACE}-dev"
    fi
    local matches=(builds/*/k8s/"${config_name}")
    if [ ! -d "${matches[0]}" ]; then
        echo "no rendered configs named ${config_name} in builds/*/k8s - run bin/k8s/generate first" >&2
        exit 1
    fi
    if [ "${#matches[@]}" -gt 1 ]; then
        echo "${config_name} is rendered under more than one build: ${matches[*]}" >&2
        exit 1
    fi
    CONFIG_DIR="${matches[0]}"
}
function deploy_lib_build_dir() {
    echo "${CONFIG_DIR%/k8s/*}"
}

ROTATED_ZONES=()

function deploy_lib_contains() {
    local needle="$1" item
    shift
    for item in "$@"; do
        [ "$item" = "$needle" ] && return 0
    done
    return 1
}

# Applies one of a zone's objects, and remembers the zone if it actually changed.
# A regenerated manifest already rolls the pods through their zone-checksum
# annotation; this covers applying without regenerating, and credential
# rotation, which the manifests cannot see at all.
function deploy_lib_apply_zone_object() {
    local zone="$1" result
    shift

    result=$(kubectl create "$@" -n "$NAMESPACE" --dry-run=client -o yaml \
        | kubectl apply -f - -n "$NAMESPACE")
    echo "${zone}: ${result}"

    case "$result" in
        *configured|*created) ;;
        *) return 0 ;;
    esac

    # A zone can get here twice, from its ConfigMap and its Secret both, and
    # restarting it twice would roll the pods twice.
    deploy_lib_contains "$zone" "${ROTATED_ZONES[@]}" || ROTATED_ZONES+=("$zone")
}
function deploy_lib_restart_rotated() {
    [ ${#ROTATED_ZONES[@]} -gt 0 ] || return 0

    echo "restarting for changed zone or credentials: ${ROTATED_ZONES[*]}"
    kubectl rollout restart -n "$NAMESPACE" \
        "${ROTATED_ZONES[@]/#/deployment/opentripplanner-}"
}

# Everything to establish before a deploy touches the cluster. The checks are
# here rather than in the deploy scripts so that the two of them, and anything
# written later, cannot disagree about what a deploy requires.
function deploy_lib_preflight() {
    deploy_lib_require_clean_tree
    deploy_lib_require_zone_files
}

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

# Fails, before anything reaches the cluster, on a zone the rendered configs
# deploy but whose files this host cannot build its ConfigMap and Secret from.
#
# gtfs-secrets.json is gitignored, so a host holding the whole source tree still
# need not have it, and one that has it can still be short a feed's entry.
function deploy_lib_require_zone_files() {
    local build_dir manifest zone zone_file secrets_file
    local no_zone=() no_secrets=() unusable_secrets=()
    build_dir=$(deploy_lib_build_dir)

    source bin/_zone-file.sh
    for manifest in "${CONFIG_DIR}"/opentripplanner-*-deployment.yaml; do
        [ -f "$manifest" ] || continue
        zone=$(basename "$manifest")
        zone=${zone#opentripplanner-}
        zone=${zone%-deployment.yaml}

        zone_file=$(zone_file_for "$build_dir" "$zone")
        if [ -z "$zone_file" ]; then
            no_zone+=("${build_dir}/transit/${zone}/zone.json")
            continue
        fi

        [ -n "$(zone_required_feeds "$zone_file" runtime)" ] || continue

        secrets_file=$(zone_secrets_file_for "$zone_file")
        if [ ! -s "$secrets_file" ]; then
            no_secrets+=("$secrets_file")
        elif ! zone_credentials_usable "$zone_file" "$secrets_file"; then
            unusable_secrets+=("$secrets_file")
        fi
    done

    if [ ${#no_zone[@]} -gt 0 ]; then
        printf '%s\n' "${no_zone[@]}" >&2
        echo "👆 Zone files these configs deploy, missing here. They are" >&2
        echo "committed, so this checkout is stale or the configs are." >&2
    fi
    if [ ${#no_secrets[@]} -gt 0 ]; then
        printf '%s\n' "${no_secrets[@]}" >&2
        echo "👆 These zones use token-gated feeds and their credentials are" >&2
        echo "missing. gtfs-secrets.json is gitignored, so it lives only where" >&2
        echo "it was written: copy it to this host, or write it here with" >&2
        echo "" >&2
        echo "    bin/transit-credentials ${build_dir}" >&2
    fi
    if [ ${#unusable_secrets[@]} -gt 0 ]; then
        printf '%s\n' "${unusable_secrets[@]}" >&2
        echo "👆 These zones have a credentials file that does not cover the" >&2
        echo "feeds they run on - see the feeds named above. Fill them into the" >&2
        echo "root gtfs-secrets.json on this host and fan it back out with" >&2
        echo "" >&2
        echo "    bin/transit-credentials ${build_dir}" >&2
    fi
    [ ${#no_zone[@]} -eq 0 ] \
        && [ ${#no_secrets[@]} -eq 0 ] \
        && [ ${#unusable_secrets[@]} -eq 0 ] || exit 1
}

# Applies a namespace's rendered configs, plus each zone's ConfigMap and Secret,
# which are held outside them so that credentials never reach the repo.
#
# The committed configs carry a placeholder artifact host; swap the real one in
# just long enough to apply, then put it back.
function deploy_lib_apply() {
    local build_dir zone_file secrets_file zone
    build_dir=$(deploy_lib_build_dir)
    ROTATED_ZONES=()

    source bin/_zone-file.sh
    for zone_file in "${build_dir}"/transit/*/zone.json; do
        [ -f "$zone_file" ] || continue
        zone=$(zone_name_for_file "$zone_file")
        secrets_file=$(zone_secrets_file_for "$zone_file")

        deploy_lib_apply_zone_object "$zone" \
            configmap "otp-${zone}-zone" --from-file="zone.json=${zone_file}"

        # A zone that reaches here without a secrets file passed the
        # preflight by not needing one.
        [ -f "$secrets_file" ] || continue
        deploy_lib_apply_zone_object "$zone" \
            secret generic "otp-${zone}-gtfs-secrets" \
            --from-file="gtfs-secrets.json=${secrets_file}"
    done

    trap deploy_lib_revert_fetch_urls EXIT
    bin/update-fetch-urls
    FETCH_URLS_UPDATED=true

    (cd "$CONFIG_DIR" && kubectl apply -f . -n "$NAMESPACE")
    deploy_lib_revert_fetch_urls
}

FETCH_URLS_UPDATED=false

# Puts the placeholder artifact host back, at most once.
function deploy_lib_revert_fetch_urls() {
    [ "$FETCH_URLS_UPDATED" = true ] || return 0
    FETCH_URLS_UPDATED=false
    bin/revert-fetch-urls
}
