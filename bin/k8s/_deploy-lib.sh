#!/bin/bash

set -o pipefail

NAMESPACE=""
CONFIG_DIR=""
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
ROTATED_DEPLOYMENTS=()
function deploy_lib_apply_transit_config() {
    local build_dir zone_file secrets_file zone missing=0
    build_dir=$(deploy_lib_build_dir)
    ROTATED_DEPLOYMENTS=()

    source bin/_zone-file.sh
    for zone_file in "${build_dir}"/transit/*/zone.json; do
        [ -f "$zone_file" ] || continue
        zone=$(zone_name_for_file "$zone_file")
        secrets_file=$(zone_secrets_file_for "$zone_file")

        deploy_lib_apply_object "$zone" \
            configmap "otp-${zone}-zone" --from-file="zone.json=${zone_file}"

        [ -f "$secrets_file" ] || continue
        deploy_lib_apply_object "$zone" \
            secret generic "otp-${zone}-gtfs-secrets" \
            --from-file="gtfs-secrets.json=${secrets_file}"
    done

    if [ "$missing" -gt 0 ]; then
        echo "Fill them in, or re-run bin/transit-credentials ${build_dir}" >&2
        exit 1
    fi
}

# Applies one generated object, and remembers the zone if it actually changed.
# A regenerated manifest already rolls the pods through their zone-checksum
# annotation; this covers applying without regenerating, and credential
# rotation, which the manifests cannot see at all.
function deploy_lib_apply_object() {
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
    local deployment="opentripplanner-${zone}"
    case " ${ROTATED_DEPLOYMENTS[*]} " in
        *" ${deployment} "*) return 0 ;;
    esac
    ROTATED_DEPLOYMENTS+=("$deployment")
}
function deploy_lib_restart_rotated() {
    [ ${#ROTATED_DEPLOYMENTS[@]} -gt 0 ] || return 0

    echo "restarting for changed zone or credentials: ${ROTATED_DEPLOYMENTS[*]}"
    kubectl rollout restart -n "$NAMESPACE" \
        "${ROTATED_DEPLOYMENTS[@]/#/deployment/}"
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
function deploy_lib_apply() {
    trap 'bin/revert-fetch-urls' EXIT
    bin/update-fetch-urls

    (cd "$CONFIG_DIR" && kubectl apply -f . -n "$NAMESPACE")
}
