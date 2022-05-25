FROM mediagis/nominatim:4.0

COPY data.osm.pbf /data.osm.pbf

RUN bash -c 'useradd -m -p ${NOMINATIM_PASSWORD} nominatim'

ENV PBF_PATH=/data.osm.pbf

RUN bash -c '/app/config.sh'

RUN bash -c '/app/init.sh'

RUN mkdir /jobs

RUN touch /var/lib/postgresql/12/main/import-finished

# This is used for a one-off data export which lets us build a photon index.
COPY import_wait_dump.sh /jobs

