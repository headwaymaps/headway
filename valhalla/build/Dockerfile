FROM gisops/valhalla:latest

USER root
RUN apt-get -y update && apt-get install -y pbzip2
WORKDIR /tiles
RUN chown valhalla /tiles
USER valhalla

RUN valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json
RUN valhalla_build_timezones > /tiles/timezones.sqlite

COPY data.osm.pbf /tiles/

RUN valhalla_build_tiles -c valhalla.json /tiles/data.osm.pbf

RUN bash -c 'cd /tiles && ls | tar -c --files-from - | pbzip2 -c -6 > /tiles/valhalla.tar.bz2'
