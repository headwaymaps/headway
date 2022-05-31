FROM gisops/valhalla:latest

USER root
WORKDIR /tiles
RUN chown valhalla /tiles
USER valhalla

RUN valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json
RUN valhalla_build_timezones > /tiles/timezones.sqlite

COPY data.osm.pbf /tiles/

RUN valhalla_build_tiles -c valhalla.json /tiles/data.osm.pbf

RUN bash -c 'cd /tiles && ls | tar -cf /tiles/valhalla.tar --files-from -'
