VERSION --use-copy-link 0.6


##############################
# OSM extract
##############################

build:
    ARG area
    ARG countries
    BUILD +save --area=${area} --countries=${countries}
    BUILD +images

save:
    ARG area
    ARG countries
    BUILD +save-extract --area=${area}
    BUILD +save-gtfs --area=${area}
    BUILD +save-otp --area=${area}
    BUILD +save-valhalla --area=${area}
    BUILD +save-elasticsearch --area=${area} --countries=${countries}
    BUILD +save-placeholder --area=${area} --countries=${countries}
    BUILD +save-interpolation --area=${area} --countries=${countries}
    BUILD +save-pelias-config --area=${area} --countries=${countries}

save-extract:
    FROM +save-base
    ARG area
    COPY (+extract/data.osm.pbf --area=${area}) /data.osm.pbf
    SAVE ARTIFACT /data.osm.pbf AS LOCAL ./data/${area}.osm.pbf

save-gtfs:
    FROM +save-base
    ARG area
    COPY (+gtfs-build/gtfs.tar --area=${area}) /gtfs.tar
    SAVE ARTIFACT /gtfs.tar AS LOCAL ./data/${area}.gtfs.tar

save-otp:
    FROM +save-base
    ARG area
    COPY (+otp-build/graph.obj --area=${area}) /graph.obj
    SAVE ARTIFACT /graph.obj AS LOCAL ./data/${area}.graph.obj

save-valhalla:
    FROM +save-base
    ARG area
    COPY (+valhalla-build/tiles --area=${area}) /valhalla
    RUN bash -c 'cd /valhalla && ls | tar -c --files-from - | pbzip2 -c -6 > /valhalla.tar.bz2'
    SAVE ARTIFACT /valhalla.tar.bz2 AS LOCAL ./data/${area}.valhalla.tar.bz2

save-elasticsearch:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-import/elasticsearch --area=${area} --countries=${countries}) /elasticsearch
    RUN bash -c 'cd /elasticsearch && ls | tar -c --files-from - | pbzip2 -c -6 > /elasticsearch.tar.bz2'
    SAVE ARTIFACT /elasticsearch.tar.bz2 AS LOCAL ./data/${area}.elasticsearch.tar.bz2

save-placeholder:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-prepare-placeholder/placeholder --area=${area} --countries=${countries}) /placeholder
    RUN bash -c 'cd /placeholder && ls | tar -c --files-from - | pbzip2 -c -6 > /placeholder.tar.bz2'
    SAVE ARTIFACT /placeholder.tar.bz2 AS LOCAL ./data/${area}.placeholder.tar.bz2

save-interpolation:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-prepare-interpolation/interpolation --area=${area} --countries=${countries}) /interpolation
    RUN bash -c 'cd /interpolation && ls | tar -c --files-from - | pbzip2 -c -6 > /interpolation.tar.bz2'
    SAVE ARTIFACT /interpolation.tar.bz2 AS LOCAL ./data/${area}.interpolation.tar.bz2

save-pelias-config:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) /pelias.json
    SAVE ARTIFACT /pelias.json AS LOCAL ./data/${area}.pelias.json


images:
    FROM debian:bullseye-slim
    COPY (+tileserver-build/fonts.tar) /fonts.tar
    COPY (+tileserver-build/sprite.tar) /sprite.tar
    SAVE ARTIFACT /fonts.tar AS LOCAL ./data/fonts.tar
    SAVE ARTIFACT /sprite.tar AS LOCAL ./data/sprite.tar
    BUILD +otp-serve-image
    BUILD +valhalla-serve-image
    BUILD +web-serve-image
    BUILD +tileserver-serve-image
    BUILD +otp-init-image
    BUILD +valhalla-init-image
    BUILD +web-init-image
    BUILD +tileserver-init-image
    BUILD +pelias-init-image

extract:
    FROM +downloader-base
    ARG area
    COPY --if-exists ${area}.osm.pbf /data/data.osm.pbf
	IF [ ! -f "/data/data.osm.pbf" ]
        RUN wget -U headway/1.0 -O /data/data.osm.pbf "https://download.bbbike.org/osm/bbbike/${area}/${area}.osm.pbf"
    END
    SAVE ARTIFACT /data/data.osm.pbf /data.osm.pbf

##############################
# Pelias
##############################

pelias-init-image:
    FROM +downloader-base
    RUN apt-get update -y && apt-get install -y --no-install-recommends pbzip2
    COPY ./services/pelias/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    SAVE IMAGE headway_pelias_init

pelias-config:
    FROM debian:bullseye-slim
    RUN apt-get update -y && apt-get install -y --no-install-recommends gettext-base
    WORKDIR /config
    COPY services/pelias/pelias.json.template pelias.json.template
    ARG countries
    ENV COUNTRIES=${countries}
    IF [ -z ${COUNTRIES} ]
        RUN echo "Must use --countries flag for custom extracts" && exit 1
    END
    RUN COUNTRY_CODE_LIST="[\"$(echo ${COUNTRIES} | sed 's/,/", "/g')\"]" \
        bash -c "envsubst < pelias.json.template > pelias.json"
    SAVE ARTIFACT /config/pelias.json /pelias.json

pelias-import-base:
    FROM earthly/dind:alpine
    ARG area
    ARG countries
    RUN mkdir -p /data/openstreetmap
    COPY (+extract/data.osm.pbf --area=${area}) /data/openstreetmap    
    WORKDIR /config
    COPY (+pelias-config/pelias.json --countries=${countries}) /config/pelias.json
    COPY services/pelias/docker-compose-import.yaml /config/compose.yaml
    ENV DATA_DIR="/data"

pelias-download-wof:
    FROM earthly/dind:alpine
    ARG countries
    RUN mkdir -p /data/openstreetmap
    WORKDIR /config
    COPY (+pelias-config/pelias.json --countries=${countries}) /config/pelias.json
    COPY services/pelias/docker-compose-import.yaml /config/compose.yaml
    ENV DATA_DIR="/data"

    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!
    WITH DOCKER \
            --compose compose.yaml \
            --service pelias_whosonfirst
        RUN docker-compose run -T 'pelias_whosonfirst' bash ./bin/download
    END
    SAVE ARTIFACT /data/whosonfirst /whosonfirst

pelias-prepare-polylines:
    ARG area
    ARG countries
    FROM +pelias-import-base
    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!
    WITH DOCKER \
            --compose compose.yaml \
            --service pelias_polylines_prepare
        RUN docker-compose run -T 'pelias_polylines_prepare' bash ./docker_extract.sh
    END
    SAVE ARTIFACT /data/polylines /polylines

pelias-prepare-interpolation:
    ARG area
    ARG countries
    FROM +pelias-import-base
    COPY (+pelias-prepare-polylines/polylines --area=${area} --countries=${countries}) /data/polylines
    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!
    WITH DOCKER \
            --compose compose.yaml \
            --service pelias_interpolation
        RUN docker-compose run -T 'pelias_interpolation' bash ./docker_build.sh
    END
    SAVE ARTIFACT /data/interpolation /interpolation

pelias-prepare-placeholder:
    ARG area
    ARG countries
    FROM +pelias-import-base
    COPY (+pelias-download-wof/whosonfirst --area=${area} --countries=${countries}) /data/whosonfirst
    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!
    WITH DOCKER \
            --compose compose.yaml \
            --service pelias_placeholder
        RUN docker-compose run -T 'pelias_placeholder' bash -c "./cmd/extract.sh && ./cmd/build.sh"
    END
    SAVE ARTIFACT /data/placeholder /placeholder

pelias-import:
    ARG area
    ARG countries
    FROM +pelias-import-base
    COPY (+pelias-download-wof/whosonfirst --area=${area} --countries=${countries}) /data/whosonfirst
    COPY (+pelias-prepare-polylines/polylines --area=${area} --countries=${countries}) /data/polylines
    RUN mkdir tools
    COPY services/pelias/wait.sh ./tools/wait.sh
    RUN mkdir /data/elasticsearch
    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!
    WITH DOCKER \
            --compose compose.yaml \
            --service pelias_schema \
            --service pelias_elasticsearch \
            --service pelias_openstreetmap \
            --service pelias_polylines_import
        RUN docker-compose run -T 'pelias_schema' bash -c "/tools/wait.sh && ./bin/create_index" && \
            docker-compose run -T 'pelias_openstreetmap' bash -c "/tools/wait.sh && ./bin/start" && \
            docker-compose run -T 'pelias_polylines_import' bash -c "/tools/wait.sh && ./bin/start"
    END
    SAVE ARTIFACT /data/elasticsearch /elasticsearch

##############################
# Planetiler
##############################

planetiler-download-mirrored-data:
    FROM +downloader-base
    WORKDIR /data
    RUN wget https://f000.backblazeb2.com/file/headway/sources.tar && tar xvf sources.tar && rm sources.tar
    SAVE ARTIFACT /data/lake_centerline.shp.zip /lake_centerline.shp.zip
    SAVE ARTIFACT /data/natural_earth_vector.sqlite.zip /natural_earth_vector.sqlite.zip
    SAVE ARTIFACT /data/water-polygons-split-3857.zip /water-polygons-split-3857.zip

planetiler-download:
    FROM +downloader-base
    ARG PLANETILER_VERSION=v0.5.0
    ARG PLANETILER_HASH=5f08d8f351751373084b1c2abd21bb38cbf66357dd2a02d2692d3561f16db70b
    
    RUN wget -O /data/planetiler.jar https://github.com/onthegomap/planetiler/releases/download/${PLANETILER_VERSION}/planetiler.jar
    RUN ls -l /data
    RUN echo "${PLANETILER_HASH}  /data/planetiler.jar" | sha256sum --check
    
    SAVE ARTIFACT /data/planetiler.jar /planetiler.jar

planetiler-image:
    FROM +java-base
    COPY +planetiler-download/planetiler.jar /planetiler/planetiler.jar
    COPY +planetiler-download-mirrored-data/lake_centerline.shp.zip /data/sources/
    COPY +planetiler-download-mirrored-data/natural_earth_vector.sqlite.zip /data/sources/
    COPY +planetiler-download-mirrored-data/water-polygons-split-3857.zip /data/sources/

planetiler-build-mbtiles:
    FROM +planetiler-image
    WORKDIR /
    ARG area
    COPY (+extract/data.osm.pbf --area=${area}) /data/
    RUN sha256sum /planetiler/planetiler.jar && java -jar /planetiler/planetiler.jar --force osm_path=/data/data.osm.pbf
    SAVE ARTIFACT /data/output.mbtiles /output.mbtiles

##############################
# GTFS
##############################

gtfs-base:
    FROM python:3
    RUN pip install requests
    WORKDIR /gtfs
    RUN mkdir /gtfs_feeds

gtfs-enumerate:
    FROM +gtfs-base
    COPY ./services/gtfs/enumerate_gtfs_feeds.py /gtfs/
    ARG area
    ARG bbox
    ENV BBOX=${bbox}
    IF [ ! -z "${BBOX}" ]
        ENV HEADWAY_BBOX=${BBOX}
        RUN python /gtfs/enumerate_gtfs_feeds.py
    ELSE
        COPY web/bboxes.csv /gtfs/bboxes.csv
        RUN bash -c "export HEADWAY_BBOX=\"$(grep "${area}:" /gtfs/bboxes.csv | cut -d':' -f2)\" && python /gtfs/enumerate_gtfs_feeds.py"
    END
    SAVE ARTIFACT /gtfs_feeds/gtfs_feeds.csv /gtfs_feeds.csv AS LOCAL ./data/${area}.gtfs_feeds.csv

gtfs-build:
    FROM +gtfs-base
    COPY ./services/gtfs/download_gtfs_feeds.py /gtfs/
    ARG area
    COPY --if-exists data/${area}.gtfs_feeds.csv /gtfs/gtfs_feeds.csv
    RUN touch /gtfs/gtfs_feeds.csv # Just in case the GTFS feeds weren't enumerated earlier.
    RUN python /gtfs/download_gtfs_feeds.py
    RUN bash -c "cd /gtfs_feeds && ls *.zip | tar -cf /gtfs/gtfs.tar --files-from -"
    SAVE ARTIFACT /gtfs/gtfs.tar /gtfs.tar

##############################
# OpenTripPlanner
##############################

otp-download:
    FROM +downloader-base

    ARG OTP_VERISON=2.1.0
    ARG OTP_HASH=b4c986b1c726c7d81d255fa183d32576122ba4e50290d53e4bb40be051971134

    RUN wget -O /data/otp-shaded.jar "https://github.com/opentripplanner/OpenTripPlanner/releases/download/v${OTP_VERISON}/otp-${OTP_VERISON}-shaded.jar"
    RUN echo "${OTP_HASH}  /data/otp-shaded.jar" | sha256sum --check

    SAVE ARTIFACT /data/otp-shaded.jar /otp-shaded.jar

otp-base:
    FROM +java11-base

    RUN mkdir /data
    RUN mkdir /otp

    COPY +otp-download/otp-shaded.jar /otp

otp-build:
    FROM +otp-base

    ARG area

    COPY (+gtfs-build/gtfs.tar --area=${area}) /data/
    COPY (+extract/data.osm.pbf --area=${area}) /data/
    WORKDIR /data
    RUN tar xvf gtfs.tar

    ARG javaMemArgs=-Xmx4G
    COPY ./services/otp/maybe_build.sh /otp

    RUN /otp/maybe_build.sh

    SAVE ARTIFACT /data/graph.obj /graph.obj

otp-init-image:
    FROM debian:bullseye-slim
    COPY ./services/otp/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    SAVE IMAGE headway_otp_init

otp-serve-image:
    FROM +otp-base

    RUN apt-get update -y && apt-get install -y --no-install-recommends netcat

    ARG javaMemArgs=-Xmx4G
    COPY ./services/otp/run_otp.sh /otp

    CMD ["/otp/run_otp.sh"]
    SAVE IMAGE headway_otp

##############################
# Valhalla
##############################

valhalla-base-image:
    FROM gisops/valhalla:latest

    USER root
    RUN apt-get -y update && apt-get install -y pbzip2
    WORKDIR /tiles
    RUN chown valhalla /tiles
    USER valhalla

valhalla-build:
    FROM +valhalla-base-image

    RUN valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json
    RUN valhalla_build_timezones > /tiles/timezones.sqlite

    ARG area
    COPY (+extract/data.osm.pbf --area=${area}) /tiles/data.osm.pbf

    RUN valhalla_build_tiles -c valhalla.json /tiles/data.osm.pbf

    SAVE ARTIFACT /tiles /tiles

valhalla-init-image:
    FROM +valhalla-base-image
    COPY ./services/valhalla/init.sh /app/init.sh
    ENTRYPOINT ["/bin/bash"]
    USER root
    CMD ["/app/init.sh"]
    SAVE IMAGE headway_valhalla_init

valhalla-serve-image:
    FROM +valhalla-base-image
    ENTRYPOINT ["valhalla_service"]
    USER valhalla
    CMD ["/data/valhalla.json"]
    SAVE IMAGE headway_valhalla

##############################
# tileserver-gl-light
##############################

tileserver-build:
    FROM node:12-slim
    RUN npm install fontnik

    COPY ./services/tileserver/assets/build_glyphs.js \
        ./services/tileserver/assets/build_sprites.js \
        ./services/tileserver/assets/package.json \
        ./services/tileserver/assets/package-lock.json \
        ./services/tileserver/assets/*.ttf \
        /app/

    WORKDIR /app
    RUN npm install
    RUN mkdir -p /app/sprite/
    COPY ./services/tileserver/assets/sprites/*.svg /app/sprite/
    WORKDIR /app
    RUN mkdir /output
    RUN useradd -ms /bin/bash fontnik
    RUN chown fontnik /output

    USER fontnik

    RUN mkdir "/output/Noto Sans Bold"
    RUN mkdir "/output/Noto Sans Italic"
    RUN mkdir "/output/Noto Sans Regular"

    RUN node build_glyphs NotoSans-Bold.ttf "/output/Noto Sans Bold"
    RUN node build_glyphs NotoSans-Italic.ttf "/output/Noto Sans Italic"
    RUN node build_glyphs NotoSans-Regular.ttf "/output/Noto Sans Regular"

    RUN node build_sprites /output/sprite /app/sprite
    RUN node build_sprites --retina /output/sprite@2x /app/sprite

    WORKDIR /output

    RUN tar -cf fonts.tar "Noto Sans Bold" "Noto Sans Italic" "Noto Sans Regular"
    RUN tar -cf sprite.tar sprite.json sprite.png sprite@2x.json sprite@2x.png

    SAVE ARTIFACT /output/fonts.tar /fonts.tar
    SAVE ARTIFACT /output/sprite.tar /sprite.tar

tileserver-init-image:
    FROM debian:bullseye-slim
    RUN apt-get update \
        && apt-get install -y --no-install-recommends ca-certificates wget pbzip2

    COPY ./services/tileserver/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    SAVE IMAGE headway_tileserver_init

tileserver-serve-image:
    FROM node:16

    RUN npm install -g tileserver-gl-light

    USER root

    RUN apt-get update -y && apt-get install -y gettext-base

    RUN mkdir -p /app/styles
    RUN mkdir -p /styles
    RUN chown -R node /app
    RUN chown -R node /styles
    USER node

    COPY ./services/tileserver/style/style.json.template /styles/style.json.template

    COPY ./services/tileserver/configure_run.sh ./services/tileserver/config.json.template /app/

    CMD ["/app/configure_run.sh"]
    SAVE IMAGE headway_tileserver

##############################
# Web
##############################

web-build:
    FROM node:16-slim
    RUN yarn global add @quasar/cli
    COPY ./web/frontend /frontend
    WORKDIR /frontend
    RUN yarn install && quasar build
    SAVE ARTIFACT /frontend/dist/spa /spa

web-init-image:
    FROM debian:bullseye-slim
    COPY ./services/nginx/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    SAVE IMAGE headway_nginx_init

web-serve-image:
    FROM nginx

    COPY web/init.sh web/bboxes.csv /frontend/

    COPY +web-build/spa /usr/share/nginx/html/

    COPY web/nginx.conf.template /etc/nginx/templates/nginx.conf.template

    ENV HEADWAY_PUBLIC_URL=http://127.0.0.1:8080
    ENV HEADWAY_BBOX_PATH=/frontend/bbox.txt
    ENV HEADWAY_CAPABILITIES_PATH=/frontend/capabilities.txt
    ENV HEADWAY_SHARED_VOL=/data
    ENV HEADWAY_HTTP_PORT=8080
    ENV HEADWAY_RESOLVER=127.0.0.11
    ENV HEADWAY_OTP_URL=http://otp:8080
    ENV HEADWAY_VALHALLA_URL=http://valhalla:8002
    ENV HEADWAY_TILESERVER_URL=http://mbtileserver:8000
    ENV HEADWAY_PELIAS_URL=http://pelias:8080
    # for escaping $ in nginx template
    ENV ESC=$
    ENV NGINX_ENVSUBST_OUTPUT_DIR=/etc/nginx
    ENTRYPOINT ["/frontend/init.sh"]

    SAVE IMAGE headway_nginx


##############################
# Generic base images
##############################

downloader-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends wget ca-certificates
    RUN mkdir /data

java-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends openjdk-17-jre-headless sudo

java11-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends openjdk-11-jre-headless sudo

save-base:
    FROM debian:bullseye-slim
    RUN apt-get -y update && apt-get install -y pbzip2
    ARG area
    ARG countries