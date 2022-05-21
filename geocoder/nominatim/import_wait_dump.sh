#!/bin/bash

useradd -m -p ${NOMINATIM_PASSWORD} nominatim

/app/config.sh

/app/init.sh

service postgresql start

sleep 5 # FIXME

sudo -u nominatim sh -c 'pg_dump nominatim > /tmp_volume/nominatim'