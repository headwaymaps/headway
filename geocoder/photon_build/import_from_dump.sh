#!/bin/bash

sudo service postgresql start && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='nominatim'" | grep -q 1 || sudo -E -u postgres createuser -s nominatim && \
sudo -E -u postgres psql postgres -tAc "SELECT 1 FROM pg_roles WHERE rolname='www-data'" | grep -q 1 || sudo -E -u postgres createuser -SDR www-data && \

# Maybe change this password? It really doesn't matter that much though, it's just ephemeral data.
sudo -E -u postgres psql postgres -tAc "ALTER USER nominatim WITH ENCRYPTED PASSWORD 'password1'" && \
sudo -E -u postgres psql postgres -tAc "ALTER USER \"www-data\" WITH ENCRYPTED PASSWORD 'password1'" && \

sudo -E -u postgres psql postgres -c "DROP DATABASE IF EXISTS nominatim"

sudo -E -u postgres psql postgres -c "CREATE DATABASE nominatim"

sudo -E -u postgres pg_restore --dbname nominatim --format tar /nominatim_data/data.nominatim.sql

sudo -u photon /bin/sh -c 'java -jar photon.jar -nominatim-import -host localhost -port 5432 -database nominatim -user nominatim -password password1 -languages es,fr,de,en'

mkdir -p /dump

tar czf /photon/photon.tgz /photon/photon_data