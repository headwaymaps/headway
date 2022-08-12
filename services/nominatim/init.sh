#!/bin/bash

# To be used for k8s init containers or run prior to each startup in docker-compose.

set -xe

if [ -f "/var/lib/postgresql/12/main/import-finished" ]; then
    echo "Nothing to do, already imported."
    exit 0
fi

if [ -f "${PBF_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cp "${PBF_SOURCE_PATH}" "${PBF_PATH}"
else
    echo "Downloading artifact"
    cd / && wget -O "${PBF_PATH}" "${PBF_URL}"
fi

if [ -f "${TOKENIZER_ARTIFACT_SOURCE_PATH}" ]; then
    echo "Copying artifact."
    cp "${TOKENIZER_ARTIFACT_SOURCE_PATH}" /nominatim_extra/tokenizer.tar
else
    echo "Downloading artifact"
    cd / && wget -O /nominatim_extra/tokenizer.tar "${TOKENIZER_ARTIFACT_URL}"
fi

service postgresql start && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='nominatim'" | grep -q 1 || sudo -E -u postgres createuser -s nominatim && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='www-data'" | grep -q 1 || sudo -E -u postgres createuser -SDR www-data && \

# Ephemeral data from a public dataset. Password can be whatever.
sudo -E -u postgres psql postgres -tAc "ALTER USER nominatim WITH ENCRYPTED PASSWORD 'password1'" && \
sudo -E -u postgres psql postgres -tAc "ALTER USER \"www-data\" WITH ENCRYPTED PASSWORD 'password1'" && \

sudo -E -u postgres psql postgres -c "DROP DATABASE IF EXISTS nominatim"

sudo -E -u postgres psql postgres -c "CREATE DATABASE nominatim"

echo "Beginning nominatim restore"

if [ -f "${DUMP_ARTIFACT_SOURCE_PATH}" ]; then
    cd / && cat "${DUMP_ARTIFACT_SOURCE_PATH}" | pbzip2 -d | sudo -E -u postgres psql nominatim
else
    cd / && wget -qO- "${DUMP_ARTIFACT_URL}" | pbzip2 -d | sudo -E -u postgres psql nominatim
fi

service postgresql stop

touch /var/lib/postgresql/12/main/import-finished

# At this point nominatim is ready for a warm start :)
