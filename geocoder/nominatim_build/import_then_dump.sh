#!/bin/bash

set -xe

service postgresql start

mkdir -p /dump

sudo -u nominatim pg_dump -F tar nominatim > /dump/nominatim.sql

cd ${PROJECT_DIR} && tar czf tokenizer.tgz tokenizer