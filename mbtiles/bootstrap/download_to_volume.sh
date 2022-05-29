#!/bin/bash

cd /data

mkdir -p sources
cd sources
ls *.zip || (curl https://f000.backblazeb2.com/file/headway/sources.tar | tar xv)
