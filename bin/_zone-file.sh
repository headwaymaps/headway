# Finding a transit zone's config and reading the realtime half of it.
#
# A zone lives at <config-dir>/transit/zones/<zone>.json, written by the feed
# picker. It carries the feeds, their credentials, and the OTP updaters that
# serve them - so everything the deployment scripts used to read out of a
# separate <Zone>-router-config.json is now a field in there.

# The zone file for a zone, or nothing if the config has none.
#
# The file name is the zone name: bin/build-transit lints it as a k8s object
# name, and everything downstream - the artifact manifest keys, the OTP
# deployment names - is that same string.
function zone_file_for() {
    local config_dir="$1"
    local zone="$2"
    local path="${config_dir}/transit/zones/${zone}.json"

    # `if` rather than `[ -f ] &&` so a missing zone isn't a nonzero status: the
    # callers assign this in a `set -e` script, where that would abort them.
    if [ -f "$path" ]; then
        echo "$path"
    fi
}

# A zone's OTP router-config.json, on stdout.
#
# jq rather than a grep: this is a nested JSON object, and the deployment pastes
# it into a ConfigMap verbatim, so it has to come out as JSON rather than as
# whatever lines happened to match.
function zone_router_config() {
    local zone_file="$1"

    if ! command -v jq > /dev/null; then
        echo "jq is required to read the realtime config out of ${zone_file}" >&2
        exit 1
    fi
    jq '.router_config // {"updaters": []}' "$zone_file"
}
