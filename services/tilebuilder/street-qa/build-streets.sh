#!/bin/sh
# Builds this checkout's tilebuilder and writes <out-dir>/streets.pmtiles: just the transportation layer, to z15.
# Usage: build-streets.sh <osm.pbf> <out-dir>
set -e
PBF=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
mkdir -p "$2"
OUT=$(cd "$2" && pwd)
T=$(cd "$(dirname "$0")/.." && pwd)
docker run --rm -v "$T":/src -v planetiler-m2:/root/.m2 maven:3.9.11-eclipse-temurin-25 \
  sh -c 'cd /src && mvn --batch-mode -q package' 2>&1 | grep -E "ERROR|FAIL|Tests run" | head -30
cp "$(ls -S "$T"/target/*.jar | head -1)" "$OUT/headway-tilebuilder.jar"
docker run --rm -v "$OUT/headway-tilebuilder.jar":/app/planetiler.jar -v "$PBF":/data/area.osm.pbf:ro -v "$OUT":/out \
  -v planetiler-sources:/data/sources -w / \
  eclipse-temurin:25-jre java -jar /app/planetiler.jar --osm_path=/data/area.osm.pbf --download --force \
  --output=/out/streets.pmtiles --only_layers=transportation --maxzoom=15 --simplify_tolerance_at_max_zoom=0 \
  --tmpdir=/out/tmp > "$OUT/build.log" 2>&1
rm -rf "$OUT/tmp"
echo "errors: $(grep -c 'Caught error postprocessing' "$OUT/build.log")"
ls -l "$OUT/streets.pmtiles"
