function zone_file_for() {
    local config_dir="$1"
    local zone="$2"
    local path="${config_dir}/transit/${zone}/zone.json"
    if [ -f "$path" ]; then
        echo "$path"
    fi
}

function zone_files_in() {
    local config_dir="$1"
    local file
    for file in "${config_dir}"/transit/*/zone.json; do
        [ -e "$file" ] && echo "$file"
    done
    return 0
}

function zone_name_for_file() {
    basename "$(dirname "$1")"
}

function zone_require_cargo() {
    if ! command -v cargo > /dev/null; then
        echo "cargo is required to read the credentials a zone needs" >&2
        exit 1
    fi
}

function zone_required_feeds() {
    local zone_file="$1"
    local scope="$2"

    zone_require_cargo
    bin/zone-router-config --zone "$zone_file" --required-feeds "$scope"
}

# Whether a zone's credentials cover every realtime feed it runs on, checked by
# rendering the router config the OTP init container renders. The rendering
# carries live tokens; the diagnostics are on stderr.
function zone_credentials_usable() {
    local zone_file="$1"
    local secrets_file="$2"

    zone_require_cargo
    bin/zone-router-config --zone "$zone_file" --credentials-file "$secrets_file" \
        > /dev/null
}

function zone_secrets_file_for() {
    echo "$(dirname "$1")/gtfs-secrets.json"
}
