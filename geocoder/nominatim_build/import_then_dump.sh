#!/bin/bash

set -xe

service postgresql start

mkdir -p /dump

sudo -u nominatim pg_dump nominatim | pbzip2 -c -6 > /dump/nominatim.sql.bz2

cd ${PROJECT_DIR} && tar czf tokenizer.tgz tokenizer
