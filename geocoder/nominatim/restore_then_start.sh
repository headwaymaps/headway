#!/bin/bash

/app/config.sh

sudo -E -u postgres pg_restore $DUMP_PATH

/app/start.sh