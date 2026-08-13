# Finding a transit zone's config and reading the realtime half of it.
#
# A zone lives at <config-dir>/transit/zones/<zone>.json, written by the feed
# picker. It carries the feeds and their credentials; the OTP updaters that
# serve them are derived from it on demand, so the separate
# <Zone>-router-config.json the deployment scripts used to read is gone.

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
# Rendered from the zone rather than read out of it: the updaters are a function
# of the realtime feeds hanging off the zone's static ones, so storing them too
# would just be a copy to keep in step. The cost is a cargo build in the deploy
# path - transit-zone is deliberately serde-and-csv only, so it's a quick one,
# and it's a no-op on every call after the first.
#
# The binary's own diagnostics (a realtime feed OTP can't be given a credential
# for) go to stderr, so what lands in the ConfigMap is only ever the JSON.
function zone_router_config() {
    local zone_file="$1"

    if ! command -v cargo > /dev/null; then
        echo "cargo is required to render the realtime config from ${zone_file}" >&2
        exit 1
    fi
    cargo run --release --quiet --package transit-zone --bin zone-router-config \
        -- --zone "$zone_file"
}
