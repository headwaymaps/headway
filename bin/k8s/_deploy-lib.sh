#!/bin/bash
# Shared setup for the deploy scripts. Sourced, not run.

set -o pipefail

NAMESPACE=""
CONFIG_DIR=""
BUILD_DIR=""

# Reads `<build-dir> [--dev]` into NAMESPACE and CONFIG_DIR. The namespace is
# the lowercased build directory name; --dev selects the `dev` config variant,
# otherwise it selects `latest`. Then cd to the repo root so caller paths resolve.
function deploy_lib_parse_args() {
    local dev=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            --dev)
                dev=true
                shift
                ;;
            *)
                if [ -z "$BUILD_DIR" ]; then
                    BUILD_DIR="${1%/}"
                else
                    echo "Unknown argument: $1" >&2
                    exit 1
                fi
                shift
                ;;
        esac
    done

    if [ -z "$BUILD_DIR" ]; then
        echo "Usage: $0 <build-dir> [--dev]"
        echo "Examples:"
        echo "  $0 builds/planet"
        echo "  $0 builds/Seattle --dev"
        echo "  $0 builds/planet --dev"
        exit 1
    fi

    cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"

    NAMESPACE=$(basename "$BUILD_DIR" | tr '[:upper:]' '[:lower:]')
    local variant=latest
    if [ "$dev" = true ]; then
        variant=dev
    fi
    CONFIG_DIR="${BUILD_DIR}/k8s/${variant}"
    if [ ! -d "$CONFIG_DIR" ]; then
        echo "no rendered configs at ${CONFIG_DIR} - run bin/k8s/generate first" >&2
        exit 1
    fi
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

# Verify gtfs secrets are preent - k8s configs will include `optional: false`
# for a zone which requires auth.
function deploy_lib_secret_required() {
    local optional
    optional=$(awk '/secretName: .*-gtfs-secrets/ { found = 1; next }
                    found && /optional:/ { print $2; exit }' "$1")
    [ "$optional" = "false" ]
}

# Fails, before anything reaches the cluster, if a zone the rendered configs
# deploy is missing a file the deploy builds its ConfigMap or Secret from.
#
# gtfs-secrets.json is gitignored, so a host with the whole source tree still
# need not have it - and the apply that follows skips a missing one in silence,
# leaving that zone's pods stuck on FailedMount for a Secret nobody created.
function deploy_lib_require_zone_files() {
    local build_dir manifest zone zone_file secrets_file
    local no_zone=() no_secrets=()
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

        deploy_lib_secret_required "$manifest" || continue
        secrets_file=$(zone_secrets_file_for "$zone_file")
        if [ ! -s "$secrets_file" ]; then
            no_secrets+=("$secrets_file")
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
    [ ${#no_zone[@]} -eq 0 ] && [ ${#no_secrets[@]} -eq 0 ] || exit 1
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

    trap 'bin/revert-fetch-urls' EXIT
    bin/update-fetch-urls

    (cd "$CONFIG_DIR" && kubectl apply -f . -n "$NAMESPACE")
}
