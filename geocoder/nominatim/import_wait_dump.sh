#!/bin/bash

useradd -m -p ${NOMINATIM_PASSWORD} nominatim

/app/config.sh

/app/init.sh

service postgresql start

sudo -u nominatim pg_dump nominatim > /tmp_volume/nominatim