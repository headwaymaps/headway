#!/bin/bash

set -xe

(cd ${PROJECT_DIR} && tar xvf ${HEADWAY_NOMINATIM_TOKENIZER})

service postgresql start && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='nominatim'" | grep -q 1 || sudo -E -u postgres createuser -s nominatim && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='www-data'" | grep -q 1 || sudo -E -u postgres createuser -SDR www-data && \

# Maybe change this password? It really doesn't matter that much though, it's just ephemeral data.
sudo -E -u postgres psql postgres -tAc "ALTER USER nominatim WITH ENCRYPTED PASSWORD 'password1'" && \
sudo -E -u postgres psql postgres -tAc "ALTER USER \"www-data\" WITH ENCRYPTED PASSWORD 'password1'" && \

sudo -E -u postgres psql postgres -c "DROP DATABASE IF EXISTS nominatim"

sudo -E -u postgres psql postgres -c "CREATE DATABASE nominatim"

echo "Beginning nominatim restore"
time cat ${HEADWAY_NOMINATIM_FILE} | pbzip2 -d | sudo -E -u postgres psql nominatim

touch /var/lib/postgresql/12/main/import-finished

/app/start.sh
