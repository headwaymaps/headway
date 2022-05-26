#!/bin/bash

service postgresql start

sudo -u nominatim pg_dump nominatim > /tmp_volume/nominatim
