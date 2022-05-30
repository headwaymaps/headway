#!/bin/bash

cd /data

mkdir -p sources
cd sources
rm -f sources.tar
ls *.zip || wget https://f000.backblazeb2.com/file/headway/sources.tar && tar xvf sources.tar
rm -f sources.tar
