FROM mediagis/nominatim:4.0

COPY data.osm.pbf /data.osm.pbf

RUN bash -c 'useradd -m -p ${NOMINATIM_PASSWORD} nominatim'

ENV PBF_PATH=/data.osm.pbf

RUN bash -c '/app/config.sh'

RUN bash -c '/app/init.sh'

RUN touch /var/lib/postgresql/12/main/import-finished

COPY import_then_dump.sh /app

RUN /app/import_then_dump.sh