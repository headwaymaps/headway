#!/bin/bash
# Shared helpers for inspecting headway's PersistentVolumeClaims. Sourced, not run.
#
# Data artifacts are immutable and every distinct version of one gets its own
# PVC, named `<service>-<area>-<data-tag>-<date>-<hash>`. "Is this volume still
# needed?" is answered by asking the cluster who references it - never by
# parsing that name. Nothing here needs to know how versions are spelled, which
# is what keeps it working when the naming changes.

HEADWAY_PVC_SELECTOR="app.kubernetes.io/part-of=headway"

KUBECTL=()

function pvc_lib_set_namespace() {
    KUBECTL=(kubectl --namespace "$1")
}

function pvc_lib_require_deps() {
    if ! command -v jq > /dev/null; then
        echo "jq is required (brew install jq)" >&2
        exit 1
    fi
    if ! "${KUBECTL[@]}" version > /dev/null 2>&1; then
        echo "can't reach the cluster; check your kubectl context and VPN" >&2
        exit 1
    fi
}

# Every claim name spoken for, whether by a workload's pod template or by a pod
# that exists right now. The latter matters during a rollout: the outgoing pod
# still holds the old claim even though no Deployment names it anymore.
function in_use_claims() {
    {
        "${KUBECTL[@]}" get deployments,statefulsets -o json \
            | jq -r '.items[].spec.template.spec.volumes[]?.persistentVolumeClaim.claimName // empty'
        "${KUBECTL[@]}" get pods -o json \
            | jq -r '.items[].spec.volumes[]?.persistentVolumeClaim.claimName // empty'
    } | sort -u
}

# name <TAB> capacity <TAB> phase <TAB> created
function headway_pvcs() {
    "${KUBECTL[@]}" get pvc -l "$HEADWAY_PVC_SELECTOR" -o json | jq -r '
        .items[]
        | [ .metadata.name,
            (.spec.resources.requests.storage // "?"),
            (.status.phase // "?"),
            (.metadata.creationTimestamp // "?") ]
        | @tsv'
}
