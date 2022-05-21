#!/bin/bash

files=(*.pgsql.tgz)

tar xvf ${files[0]}

export PGPASSWORD=god

psql -h postgis -U postgis -d postgis -a -f ./load.sql