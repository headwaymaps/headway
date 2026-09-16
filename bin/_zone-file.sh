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

function zone_required_feeds() {
    local zone_file="$1"
    local scope="$2"

    if ! command -v cargo > /dev/null; then
        echo "cargo is required to read the credentials ${zone_file} needs" >&2
        exit 1
    fi
    bin/zone-router-config --zone "$zone_file" --required-feeds "$scope"
}

function zone_secrets_file_for() {
    echo "$(dirname "$1")/gtfs-secrets.json"
}
