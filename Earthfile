VERSION --use-copy-link 0.6


##############################
# OSM extract
##############################

build:
    # The name of <area>.osm.pbf if you've downloaded a custom extract, or the
    # name of one of the pre-configured downloadable extracts available from
    # bbike.org
    ARG area

    # `countries is uses by whosonfirst dataset.
    # If left blank we try to guess the country based on the area argument.
    # Use the special value `ALL` when doing a planet build.
    ARG countries

    # tag for created docker containers
    ARG tag="latest"

    # Run +gtfs-enumerate to build an appropriate input for transit_feeds.
    # If omitted, you cannot enable transit routing.
    ARG transit_feeds

    BUILD +save --area=${area} --countries=${countries} --transit_feeds=${transit_feeds}
    BUILD +images --tag=${tag}

save:
    FROM +save-base
    ARG area
    ARG countries
    ARG transit_feeds
    BUILD +save-extract --area=${area}
    BUILD +save-mbtiles --area=${area}
    IF [ ! -z "${transit_feeds}" ]
        BUILD +save-gtfs --area=${area} --transit_feeds=${transit_feeds}
        BUILD +save-otp --area=${area} --transit_feeds=${transit_feeds}
    END
    BUILD +save-valhalla --area=${area}
    BUILD +save-elasticsearch --area=${area} --countries=${countries}
    BUILD +save-placeholder --area=${area} --countries=${countries}
    BUILD +save-pelias-config --area=${area} --countries=${countries}
    BUILD +save-tileserver-natural-earth

save-polylines:
    FROM +save-base
    ARG area
    RUN mkdir -p /data
    COPY (+valhalla-build-polylines/polylines.0sv --area=${area}) /data/polylines.0sv
    # This isn't used at runtime, but it can be useful when doing a
    # planet-scale import of pelias outside of earthly.
    SAVE ARTIFACT /data/polylines.0sv AS LOCAL ./data/${area}-polylines.0sv

save-extract:
    FROM +save-base
    ARG area
    COPY (+extract/data.osm.pbf --area=${area}) /data.osm.pbf
    # This isn't used at runtime, but it might be useful to archive the input
    SAVE ARTIFACT /data.osm.pbf AS LOCAL ./data/${area}.osm.pbf

save-gtfs:
    FROM +save-base
    ARG --required area
    ARG --required transit_feeds
    COPY (+gtfs-build/gtfs.tar.xz --transit_feeds=${transit_feeds}) /gtfs.tar.xz
    # This isn't used at runtime, but it might be useful to archive the input
    SAVE ARTIFACT /gtfs.tar.xz AS LOCAL ./data/${area}.gtfs.tar.xz

save-otp:
    FROM +save-base
    ARG --required area
    ARG --required transit_feeds

    COPY (+otp-build/graph.obj --area=${area} --transit_feeds=${transit_feeds}) /graph.obj
    RUN xz /graph.obj
    SAVE ARTIFACT /graph.obj.xz AS LOCAL ./data/${area}.graph.obj.xz

save-mbtiles:
    FROM +save-base
    ARG area
    COPY (+planetiler-build-mbtiles/output.mbtiles --area=${area}) /output.mbtiles
    SAVE ARTIFACT /output.mbtiles AS LOCAL ./data/${area}.mbtiles

save-valhalla:
    FROM +save-base
    ARG area
    COPY (+valhalla-build/tiles --area=${area}) /valhalla
    RUN tar -cJf /valhalla.tar.xz -C /valhalla .
    SAVE ARTIFACT /valhalla.tar.xz AS LOCAL ./data/${area}.valhalla.tar.xz

save-elasticsearch:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-import/elasticsearch --area=${area} --countries=${countries}) /elasticsearch
    RUN tar -cJf /elasticsearch.tar.xz -C /elasticsearch .
    SAVE ARTIFACT /elasticsearch.tar.xz AS LOCAL ./data/${area}.elasticsearch.tar.xz

save-placeholder:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-prepare-placeholder/placeholder --area=${area} --countries=${countries}) /placeholder
    RUN tar -cJf /placeholder.tar.xz -C /placeholder .
    SAVE ARTIFACT /placeholder.tar.xz AS LOCAL ./data/${area}.placeholder.tar.xz

save-pelias-config:
    FROM +save-base
    ARG area
    ARG countries
    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) /pelias.json
    SAVE ARTIFACT /pelias.json AS LOCAL ./data/${area}.pelias.json

save-tileserver-natural-earth:
    FROM +downloader-base
    RUN wget -nv https://publicdata.ellenhp.workers.dev/natural_earth_2_shaded_relief.raster.mbtiles
    SAVE ARTIFACT natural_earth_2_shaded_relief.raster.mbtiles AS LOCAL ./data/natural_earth.mbtiles

images:
    FROM debian:bullseye-slim
    ARG tag="latest"
    ARG branding
    COPY (+tileserver-build/fonts.tar) /fonts.tar
    COPY (+tileserver-build/sprite.tar) /sprite.tar
    SAVE ARTIFACT /fonts.tar AS LOCAL ./data/fonts.tar
    SAVE ARTIFACT /sprite.tar AS LOCAL ./data/sprite.tar
    BUILD +otp-serve-image --tag=${tag}
    BUILD +valhalla-serve-image --tag=${tag}
    BUILD +web-serve-image --tag=${tag} --branding=${branding}
    BUILD +tileserver-serve-image --tag=${tag}
    BUILD +otp-init-image --tag=${tag}
    BUILD +valhalla-init-image --tag=${tag}
    BUILD +web-init-image --tag=${tag}
    BUILD +tileserver-init-image --tag=${tag}
    BUILD +pelias-init-image --tag=${tag}

extract:
    FROM +downloader-base
    ARG area
    COPY --if-exists ${area}.osm.pbf /data/data.osm.pbf
    IF [ ! -f "/data/data.osm.pbf" ]
        RUN wget -nv -U headway/1.0 -O /data/data.osm.pbf "https://download.bbbike.org/osm/bbbike/${area}/${area}.osm.pbf"
    END
    SAVE ARTIFACT /data/data.osm.pbf /data.osm.pbf

##############################
# Pelias
##############################

pelias-init-image:
    FROM +downloader-base
    RUN mkdir -p /app
    COPY ./services/pelias/init* /app/
    CMD ["echo", "run a specific command"]
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/pelias-init:${tag}

pelias-guess-country:
    FROM debian:bullseye-slim
    COPY services/pelias/cities_to_countries.csv /data/cities_to_countries.csv
    ARG area
    ENV HEADWAY_AREA=${area}
    RUN grep "^${HEADWAY_AREA}:" /data/cities_to_countries.csv | cut -d':' -f2 > /data/guessed_country
    SAVE ARTIFACT /data/guessed_country /guessed_country

# We use this both for import and for production pelias instances.
# But we might want to try a longer timeout for the import process?
pelias-config:
    FROM debian:bullseye-slim
    RUN apt-get update \
        && apt-get install -y --no-install-recommends gettext-base \
        && rm -rf /var/lib/apt/lists/*
    WORKDIR /config
    COPY services/pelias/pelias.json.template pelias.json.template
    ARG countries
    ARG area
    ENV COUNTRIES=${countries}
    IF [ -z ${COUNTRIES} ]
        COPY (+pelias-guess-country/guessed_country --area=${area}) guessed_country
        IF [ -s guessed_country ]
            RUN echo "Using guessed country"
            RUN COUNTRY_CODE_LIST="[\"$(cat guessed_country | sed 's/,/", "/g')\"]" \
                bash -c "envsubst < pelias.json.template > pelias.json"
        ELSE
            RUN echo "Must use --countries flag for custom extracts" && exit 1
        END
    ELSE
        IF [ "$COUNTRIES" = "ALL" ]
            # Special-case the whole planet.
            RUN sed '/COUNTRY_CODE_LIST/d' pelias.json.template > pelias.json
            RUN cat pelias.json
        ELSE
            RUN COUNTRY_CODE_LIST="[\"$(echo ${COUNTRIES} | sed 's/,/", "/g')\"]" \
                bash -c "envsubst < pelias.json.template > pelias.json"
        END
    END
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
    RUN mkdir -p /data/polylines
    COPY (+valhalla-build-polylines/polylines.0sv --area=${area}) /data/polylines/extract.0sv
    SAVE ARTIFACT /data/polylines /polylines

pelias-prepare-placeholder:
    ARG area
    ARG countries
    FROM +pelias-import-base
    COPY (+pelias-download-wof/whosonfirst --countries=${countries}) /data/whosonfirst
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
    COPY (+pelias-download-wof/whosonfirst --countries=${countries}) /data/whosonfirst
    COPY (+pelias-prepare-polylines/polylines --area=${area} --countries=${countries}) /data/polylines
    RUN mkdir tools
    COPY services/pelias/wait.sh ./tools/wait.sh
    RUN mkdir /data/elasticsearch
    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!

    WITH DOCKER --compose compose.yaml --service pelias_schema
        RUN docker-compose run -T 'pelias_schema' bash -c "/tools/wait.sh && ./bin/create_index"
    END

    WITH DOCKER --compose compose.yaml --service pelias_openstreetmap
        RUN docker-compose run -T 'pelias_openstreetmap' bash -c "/tools/wait.sh && ./bin/start"
    END

    # This usually fails for planet builts due to: https://github.com/pelias/docker/issues/217
    # Interestingly it usually (always?) succeeds for smaller builds like Seattle.
    #
    # For production, I've done a manual import of the planet data including WOF based on
    # manually running the steps in https://github.com/pelias/docker/tree/master/projects/planet
    # I was actually seeing the same error initially, and in the process of debugging, but then
    # it just succeeded on the billionth attempt, so I decided we might as well use the artifact
    # for now while waiting for a proper fix.
    WITH DOCKER --compose compose.yaml --service pelias_whosonfirst
        RUN docker-compose run -T 'pelias_whosonfirst' bash -c "/tools/wait.sh && ./bin/start"
    END

    WITH DOCKER --compose compose.yaml --service pelias_polylines_import
        RUN docker-compose run -T 'pelias_polylines_import' bash -c "/tools/wait.sh && ./bin/start"
    END

    SAVE ARTIFACT /data/elasticsearch /elasticsearch

##############################
# Planetiler
##############################

planetiler-download-mirrored-data:
    FROM +downloader-base
    WORKDIR /data
    RUN wget -nv https://f000.backblazeb2.com/file/headway/sources.tar && tar xvf sources.tar && rm sources.tar
    SAVE ARTIFACT /data/lake_centerline.shp.zip /lake_centerline.shp.zip
    SAVE ARTIFACT /data/natural_earth_vector.sqlite.zip /natural_earth_vector.sqlite.zip
    SAVE ARTIFACT /data/water-polygons-split-3857.zip /water-polygons-split-3857.zip

planetiler-download:
    FROM +downloader-base
    ARG PLANETILER_VERSION=v0.5.0
    ARG PLANETILER_HASH=5f08d8f351751373084b1c2abd21bb38cbf66357dd2a02d2692d3561f16db70b
    
    RUN wget -nv -O /data/planetiler.jar https://github.com/onthegomap/planetiler/releases/download/${PLANETILER_VERSION}/planetiler.jar
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
    ARG --required transit_feeds

    COPY "${transit_feeds}" /gtfs/gtfs_feeds.csv

    COPY ./services/gtfs/download_gtfs_feeds.py /gtfs/
    RUN python /gtfs/download_gtfs_feeds.py
    RUN cd /gtfs_feeds && ls *.zip | tar -cJf /gtfs/gtfs.tar.xz --files-from -
    SAVE ARTIFACT /gtfs/gtfs.tar.xz /gtfs.tar.xz

##############################
# OpenTripPlanner
##############################

otp-download:
    FROM +downloader-base

    ARG OTP_VERISON=2.1.0
    ARG OTP_HASH=b4c986b1c726c7d81d255fa183d32576122ba4e50290d53e4bb40be051971134

    RUN wget -nv -O /data/otp-shaded.jar "https://github.com/opentripplanner/OpenTripPlanner/releases/download/v${OTP_VERISON}/otp-${OTP_VERISON}-shaded.jar"
    RUN echo "${OTP_HASH}  /data/otp-shaded.jar" | sha256sum --check

    SAVE ARTIFACT /data/otp-shaded.jar /otp-shaded.jar

otp-base:
    FROM +java11-base

    RUN mkdir /data
    RUN mkdir /otp

    COPY +otp-download/otp-shaded.jar /otp

otp-build:
    FROM +otp-base

    ARG --required transit_feeds
    ARG --required area

    COPY (+gtfs-build/gtfs.tar.xz --transit_feeds=${transit_feeds}) /data/
    COPY (+extract/data.osm.pbf --area=${area}) /data/

    WORKDIR /data
    RUN tar xvJf gtfs.tar.xz
    RUN java -Xmx4G -jar /otp/otp-shaded.jar --build --save .

    SAVE ARTIFACT /data/graph.obj /graph.obj

otp-init-image:
    FROM +downloader-base
    COPY ./services/otp/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/opentripplanner-init:${tag}

otp-serve-image:
    FROM +otp-base

    CMD java -Xmx4G -jar /otp/otp-shaded.jar --load /data
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/opentripplanner:${tag}

##############################
# Valhalla
##############################

valhalla-base-image:
    FROM michaelkirk/valhalla:date-2022-10-10

    USER root
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

valhalla-build-polylines:
    FROM +valhalla-build

    RUN valhalla_export_edges valhalla.json > /tiles/polylines.0sv

    SAVE ARTIFACT /tiles/polylines.0sv

valhalla-init-image:
    FROM +valhalla-base-image
    USER root
    RUN apt-get update \
        && apt-get install -y --no-install-recommends ca-certificates wget \
        && rm -rf /var/lib/apt/lists/*

    USER valhalla
    COPY ./services/valhalla/init.sh /app/init.sh
    ENTRYPOINT ["/bin/bash"]
    USER root
    CMD ["/app/init.sh"]
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/valhalla-init:${tag}

valhalla-serve-image:
    FROM +valhalla-base-image
    ENTRYPOINT ["valhalla_service"]
    USER valhalla
    CMD ["/data/valhalla.json"]
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/valhalla:${tag}

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

    RUN mkdir "/output/Roboto Regular"
    RUN mkdir "/output/Roboto Medium"
    RUN mkdir "/output/Roboto Condensed Italic"

    RUN node build_glyphs Roboto-Medium.ttf "/output/Roboto Medium"
    RUN node build_glyphs Roboto-Condensed-Italic.ttf "/output/Roboto Condensed Italic"
    RUN node build_glyphs Roboto-Regular.ttf "/output/Roboto Regular"

    RUN node build_sprites /output/sprite /app/sprite
    RUN node build_sprites --retina /output/sprite@2x /app/sprite

    WORKDIR /output

    RUN tar -cf fonts.tar "Roboto Medium" "Roboto Condensed Italic" "Roboto Regular"
    RUN tar -cf sprite.tar sprite.json sprite.png sprite@2x.json sprite@2x.png

    SAVE ARTIFACT /output/fonts.tar /fonts.tar
    SAVE ARTIFACT /output/sprite.tar /sprite.tar

tileserver-init-image:
    FROM debian:bullseye-slim
    RUN apt-get update \
        && apt-get install -y --no-install-recommends ca-certificates wget \
        && rm -rf /var/lib/apt/lists/*

    COPY ./services/tileserver/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/tileserver-init:${tag}

tileserver-serve-image:
    FROM node:16

    RUN npm install -g tileserver-gl-light

    USER root

    RUN apt-get update \
        && apt-get install -y gettext-base \
        && rm -rf /var/lib/apt/lists/*

    RUN mkdir -p /app/styles
    RUN mkdir -p /styles
    RUN chown -R node /app
    RUN chown -R node /styles
    USER node

    COPY ./services/tileserver/style/style.json.template /styles/style.json.template

    COPY ./services/tileserver/configure_run.sh ./services/tileserver/config.json.template /app/

    CMD ["/app/configure_run.sh"]
    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/tileserver:${tag}

##############################
# Web
##############################

web-build:
    FROM node:16-slim
    RUN yarn global add @quasar/cli
    COPY ./web/frontend /frontend
    WORKDIR /frontend
    ARG branding
    IF [ ! -z ${branding} ]
        RUN sed -i "s/.*productName.*/  \"productName\": \"${branding}\",/" package.json
    END
    RUN yarn install && quasar build
    SAVE ARTIFACT /frontend/dist/spa /spa

web-init-image:
    FROM +downloader-base

    COPY ./services/nginx/init.sh ./services/nginx/generate_config.sh /app/
    ENV HEADWAY_SHARED_VOL=/data
    CMD ["/app/init.sh"]

    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/headway-init:${tag}

web-serve-image:
    FROM nginx

    ARG branding
    COPY (+web-build/spa --branding=${branding}) /usr/share/nginx/html/

    COPY services/nginx/nginx.conf.template /etc/nginx/templates/nginx.conf.template

    ENV HEADWAY_PUBLIC_URL=http://127.0.0.1:8080
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

    ARG tag
    SAVE IMAGE --push ghcr.io/michaelkirk/headway:${tag}

##############################
# Generic base images
##############################

downloader-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends wget ca-certificates xz-utils \
        && rm -rf /var/lib/apt/lists/*
    RUN mkdir /data

java-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends openjdk-17-jre-headless sudo \
        && rm -rf /var/lib/apt/lists/*

java11-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends openjdk-11-jre-headless sudo xz-utils \
        && rm -rf /var/lib/apt/lists/*

save-base:
    FROM debian:bullseye-slim
    RUN apt-get update \
        && apt-get install -y --no-install-recommends xz-utils \
        && rm -rf /var/lib/apt/lists/*
