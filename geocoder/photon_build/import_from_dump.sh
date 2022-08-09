#!/bin/bash

set -xe

HEADWAY_NOMINATIM_FILE=${HEADWAY_NOMINATIM_FILE:-/nominatim_data/data.nominatim.sql.bz2}
HEADWAY_PHOTON_LANGUAGES=${HEADWAY_PHOTON_LANGUAGES:-es,fr,de,en}

service postgresql start && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='nominatim'" | grep -q 1 || sudo -E -u postgres createuser -s nominatim && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='www-data'" | grep -q 1 || sudo -E -u postgres createuser -SDR www-data && \

# Maybe change this password? It really doesn't matter that much though, it's just ephemeral data.
sudo -E -u postgres psql postgres -tAc "ALTER USER nominatim WITH ENCRYPTED PASSWORD 'password1'" && \
sudo -E -u postgres psql postgres -tAc "ALTER USER \"www-data\" WITH ENCRYPTED PASSWORD 'password1'" && \

sudo -E -u postgres psql postgres -c "DROP DATABASE IF EXISTS nominatim"

sudo -E -u postgres psql postgres -c "CREATE DATABASE nominatim"

cat ${HEADWAY_NOMINATIM_FILE} | pbzip2 -d | sudo -E -u postgres psql nominatim

sudo -u photon /bin/sh -c "java -jar /photon/photon.jar -nominatim-import -host localhost -port 5432 -database nominatim -user nominatim -password password1 -languages ${HEADWAY_PHOTON_LANGUAGES}"

mkdir -p /dump

tar cvf - /photon/photon_data | pbzip2 -c -6 > /photon/photon.tar.bz2
