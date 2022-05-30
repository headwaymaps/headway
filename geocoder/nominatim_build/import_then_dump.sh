#!/bin/bash

service postgresql start

mkdir -p /dump

sudo -u nominatim pg_dump -F tar nominatim > /dump/nominatim.sql
