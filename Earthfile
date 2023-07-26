VERSION --use-copy-link 0.6


##############################
# OSM extract
##############################

build:
    # The name of <area>.osm.pbf if you've downloaded a custom extract, or the
    # name of one of the pre-configured downloadable extracts available from
    # bbike.org
    ARG --required area

    # `countries is uses by whosonfirst dataset.
    # If left blank we try to guess the country based on the area argument.
    # Use the special value `ALL` when doing a planet build.
    ARG countries

    # tag for created docker containers
    ARG tags="dev"

    # Run +gtfs-enumerate to build an appropriate input for transit_feeds.
    # If omitted, you cannot enable transit routing.
    ARG transit_feeds

    BUILD +save --area=${area} --countries=${countries} --transit_feeds=${transit_feeds}
    BUILD +images --tags=${tags}

save:
    FROM +save-base
    ARG --required area
    ARG countries
    ARG transit_feeds
    BUILD +save-extract --area=${area}
    BUILD +save-mbtiles --area=${area}
    IF [ ! -z "${transit_feeds}" ]
        BUILD +save-gtfs --area=${area} --transit_feeds=${transit_feeds}
        BUILD +save-otp --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=0
    END
    BUILD +save-valhalla --area=${area}
    BUILD +save-elasticsearch --area=${area} --countries=${countries}
    BUILD +save-placeholder --area=${area} --countries=${countries}
    BUILD +save-pelias-config --area=${area} --countries=${countries}
    BUILD +save-tileserver-natural-earth

save-polylines:
    FROM +save-base
    ARG --required area
    RUN mkdir -p /data
    COPY (+valhalla-build-polylines/polylines.0sv --area=${area}) /data/polylines.0sv
    # This isn't used at runtime, but it can be useful when doing a
    # planet-scale import of pelias outside of earthly.
    SAVE ARTIFACT /data/polylines.0sv AS LOCAL ./data/${area}-polylines.0sv

save-extract:
    FROM +save-base
    ARG --required area
    COPY (+extract/data.osm.pbf --area=${area}) /data.osm.pbf
    # This isn't used at runtime, but it might be useful to archive the input
    SAVE ARTIFACT /data.osm.pbf AS LOCAL ./data/${area}.osm.pbf

save-transit-zones:
    ARG --required area
    ARG --required transit_zones
    ARG otp_build_config
    BUILD +save-gtfs-zones --area=${area} --transit_zones=${transit_zones}
    BUILD +save-otp-zones --area=${area} --transit_zones=${transit_zones} --otp_build_config=${otp_build_config}

save-gtfs-zones:
    FROM +save-base
    ARG --required area
    ARG --required transit_zones
    FOR transit_feeds IN $transit_zones
        BUILD +save-gtfs --area=${area} --transit_feeds=${transit_feeds}
    END

save-gtfs:
    FROM +save-base
    ARG --required area
    ARG --required transit_feeds

    COPY +cache-buster/todays_date .
    ARG cache_key=$(cat todays_date)

    ARG output_prefix=$(basename $transit_feeds .gtfs_feeds.csv)

    COPY (+gtfs-build/gtfs.tar.zst --transit_feeds=${transit_feeds} --cache_key=${cache_key}) /gtfs.tar.zst
    # This isn't used at runtime, but it might be useful to archive the input
    SAVE ARTIFACT /gtfs.tar.zst AS LOCAL ./data/${area}-${output_prefix}-${cache_key}.gtfs.tar.zst

save-otp-zones:
    FROM +save-base
    ARG --required area
    ARG --required transit_zones
    ARG otp_build_config
    FOR transit_feeds IN $transit_zones
        BUILD +save-otp --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=1 --otp_build_config=${otp_build_config}
        BUILD +save-otp-router-config --transit_feeds=${transit_feeds}
    END

save-otp:
    FROM +save-base
    ARG --required area
    ARG --required transit_feeds
    ARG --required clip_to_gtfs
    ARG otp_build_config

    # When working with a very large (e.g. planet sized) osm.pbf, we can't support
    # transit for the entire thing, but we can support smaller transit zones within the
    # planet.
    # We extract a bbox'd area of the input osm.pbf around the actual transit
    # zone for OTP to have any chance of fitting into memory.
    ARG transit_zone=$(basename $transit_feeds .gtfs_feeds.csv)
    IF [ -n "$clip_to_gtfs" ]
        ARG output_name="${area}-${transit_zone}"
    ELSE
        ARG clip_to_gtfs=0
        ARG output_name="${transit_zone}"
    END

    COPY +cache-buster/todays_date .
    ARG cache_key=$(cat todays_date)

    COPY (+otp-build/graph.obj --area=${area} \
                               --clip_to_gtfs=${clip_to_gtfs} \
                               --transit_feeds=${transit_feeds} \
                               --cache_key=${cache_key} \
                               --otp_build_config=${otp_build_config} \
    ) /graph.obj

    RUN zstd /graph.obj
    SAVE ARTIFACT /graph.obj.zst AS LOCAL ./data/${output_name}-${cache_key}.graph.obj.zst

save-mbtiles:
    FROM +save-base
    ARG --required area
    COPY (+planetiler-build-mbtiles/output.mbtiles --area=${area}) /output.mbtiles
    SAVE ARTIFACT /output.mbtiles AS LOCAL ./data/${area}.mbtiles

save-valhalla:
    FROM +save-base
    ARG --required area
    COPY (+valhalla-build/tiles --area=${area}) /valhalla
    RUN tar --zstd -cf /valhalla.tar.zst -C /valhalla .
    SAVE ARTIFACT /valhalla.tar.zst AS LOCAL ./data/${area}.valhalla.tar.zst

save-elasticsearch:
    FROM +save-base
    ARG --required area
    ARG countries
    COPY (+pelias-import/elasticsearch --area=${area} --countries=${countries}) /elasticsearch
    RUN tar --zstd -cf /elasticsearch.tar.zst -C /elasticsearch .
    SAVE ARTIFACT /elasticsearch.tar.zst AS LOCAL ./data/${area}.elasticsearch.tar.zst

save-placeholder:
    FROM +save-base
    ARG --required area
    ARG countries
    COPY (+pelias-prepare-placeholder/placeholder --countries=${countries}) /placeholder
    RUN tar --zstd -cf /placeholder.tar.zst -C /placeholder .
    SAVE ARTIFACT /placeholder.tar.zst AS LOCAL ./data/${area}.placeholder.tar.zst

save-pelias-config:
    FROM +save-base
    ARG --required area
    ARG countries
    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) /pelias.json
    SAVE ARTIFACT /pelias.json AS LOCAL ./data/${area}.pelias.json

save-tileserver-natural-earth:
    FROM +downloader-base
    RUN wget -nv https://publicdata.ellenhp.workers.dev/natural_earth_2_shaded_relief.raster.mbtiles
    SAVE ARTIFACT natural_earth_2_shaded_relief.raster.mbtiles AS LOCAL ./data/natural_earth.mbtiles

images:
    FROM debian:bullseye-slim
    ARG tags="dev"
    ARG branding
    BUILD +transitmux-serve-image --tags=${tags}
    BUILD +otp-serve-image --tags=${tags}
    BUILD +valhalla-serve-image --tags=${tags}
    BUILD +web-serve-image --tags=${tags} --branding=${branding}
    BUILD +tileserver-serve-image --tags=${tags}
    BUILD +otp-init-image --tags=${tags}
    BUILD +valhalla-init-image --tags=${tags}
    BUILD +web-init-image --tags=${tags}
    BUILD +tileserver-init-image --tags=${tags}
    BUILD +pelias-init-image --tags=${tags}

extract:
    FROM +downloader-base
    ARG --required area
    ARG clip_bbox

    RUN apt-get update \
        && apt-get install -y --no-install-recommends osmium-tool \
        && rm -rf /var/lib/apt/lists/*

    WORKDIR /data

    COPY --if-exists ${area}.osm.pbf data.osm.pbf
    IF [ ! -f data.osm.pbf ]
        RUN wget -nv -U headway/1.0 -O data.osm.pbf "https://download.bbbike.org/osm/bbbike/${area}/${area}.osm.pbf"
    END

    IF [ ! -f data.osm.pbf ]
        RUN echo "osm file not found"
        RUN exit 1
    END

    IF [ -n "${clip_bbox}" ]
        # I don't understand why the following line doesn't work:
        #    ARG comma_separated_bbox=$(echo ${clip_bbox} | sed 's/ /,/g')
        # ... but anway, here's a 2-line work around:
        RUN echo ${clip_bbox} | sed 's/ /,/g' > comma_separated_bbox.txt
        ARG comma_separated_bbox=$(cat comma_separated_bbox.txt)

        # It'd be nice to mv rather than cp, but I get this weird error:
        # >    mv: cannot move 'data.osm.pbf' to a subdirectory of itself, 'unclipped.osm.pbf'
        # I'm not sure if this is a bug with large files+docker+zfs or what.
        # RUN mv data.osm.pbf unclipped.osm.pbf && \
        RUN cp data.osm.pbf unclipped.osm.pbf && rm data.osm.pbf && \
            osmium extract --bbox="$comma_separated_bbox" unclipped.osm.pbf --output=data.osm.pbf && \
            rm unclipped.osm.pbf
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
    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/pelias-init:${tag}
    END

pelias-guess-country:
    FROM debian:bullseye-slim
    COPY services/pelias/cities_to_countries.csv /data/cities_to_countries.csv
    ARG --required area
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
    ARG --required area
    ENV COUNTRIES=${countries}
    IF [ -z ${COUNTRIES} ]
        COPY (+pelias-guess-country/guessed_country --area=${area}) guessed_country
        IF [ -s guessed_country ]
            RUN echo "Using guessed country $(cat guessed_country)"
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
    ARG --required area
    ARG countries
    RUN mkdir -p /data/openstreetmap
    COPY (+extract/data.osm.pbf --area=${area}) /data/openstreetmap
    WORKDIR /config
    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) /config/pelias.json
    COPY services/pelias/docker-compose-import.yaml /config/compose.yaml
    ENV DATA_DIR="/data"

pelias-download-wof:
    FROM earthly/dind:alpine
    ARG --required area
    ARG countries
    RUN mkdir -p /data/openstreetmap
    WORKDIR /config
    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) /config/pelias.json
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
    ARG --required area
    FROM +pelias-import-base
    RUN chmod -R 777 /data # FIXME: not everything should have execute permissions!
    RUN mkdir -p /data/polylines
    COPY (+valhalla-build-polylines/polylines.0sv --area=${area}) /data/polylines/extract.0sv
    SAVE ARTIFACT /data/polylines /polylines

pelias-prepare-placeholder:
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
    ARG --required area
    ARG countries
    FROM +pelias-import-base
    COPY (+pelias-download-wof/whosonfirst --countries=${countries}) /data/whosonfirst
    COPY (+pelias-prepare-polylines/polylines --area=${area}) /data/polylines
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

planetiler-base:
    # The version tag is ignored when sha256 is specified, but I'm leaving it in as documentation
    FROM ghcr.io/onthegomap/planetiler:0.5.0@sha256:79981c8af5330b384599e34d90b91a2c01b141be2c93a53244d14c49e2758c3c
    # FIXME: The 0.6.0 release is failing on planet builds (reproduced on daylight maps v1.26 w/ building and admin)
    # The 0.5.0 release builds it successfully. The 0.6.0 release can build a smaller map (e.g. Seattle) without error.
    # Failing with:
    #     java.util.concurrent.ExecutionException: java.io.UncheckedIOException: com.google.protobuf.InvalidProtocolBufferException: Protocol message contained an invalid tag (zero).
    # FROM ghcr.io/onthegomap/planetiler:0.6.0@sha256:e937250696efc60f57e7952180645c6e4b1888d70fd61d04f1e182c5489eaa1c
    SAVE IMAGE planetiler-base:latest

planetiler-build-mbtiles:
    FROM earthly/dind:alpine

    COPY +planetiler-download-mirrored-data/lake_centerline.shp.zip /data/sources/
    COPY +planetiler-download-mirrored-data/natural_earth_vector.sqlite.zip /data/sources/
    COPY +planetiler-download-mirrored-data/water-polygons-split-3857.zip /data/sources/

    ARG --required area
    COPY (+extract/data.osm.pbf --area=${area}) /data/

    # Instead of a docker-in-docker thing here, we could extend from the planetiler base image,
    # but the Entrypoint feels a little strange to hardcode since it's not a typical binary.
    # Presumably this is some automated java+docker build thing.
    # "Entrypoint": [
    #     "java",
    #     "-cp",
    #     "@/app/jib-classpath-file",
    #     "com.onthegomap.planetiler.Main"
    # ],
    WITH DOCKER --load planetiler-base:latest=+planetiler-base
        RUN docker run -v=/data:/data planetiler-base:latest --force --osm_path=/data/data.osm.pbf
    END

    SAVE ARTIFACT /data/output.mbtiles /output.mbtiles

##############################
# GTFS
##############################

gtfs-base:
    FROM python:3
    RUN pip install requests
    WORKDIR /gtfs
    RUN mkdir /gtfs_feeds

cache-buster:
    FROM +gtfs-base
    RUN --no-cache echo $(date +%Y-%m-%d) > todays_date
    SAVE ARTIFACT todays_date

gtfs-enumerate:
    FROM +gtfs-base

    COPY ./services/gtfs/filter_feeds.py /gtfs/

    # Earthly caches computed ARGs - subsequent runs will use the cache_key
    # that was computed last time, unless something else has busted the cache.
    #
    # Reported: https://github.com/earthly/earthly/issues/2523
    #
    # This is "expected behavior", but earthly might one day offer a better
    # solution such as `ARG --no-cache`. In the meanwhile we have this
    # cache-buster work around
    COPY +cache-buster/todays_date .
    ARG cache_key=$(cat todays_date)
    # If Earthly does one day implement `ARG --no-cache`, we can replace the
    # two lines above with the following:
    #    ARG --no-cache cache_key=$(date +%Y-%m-%d)

    COPY (+gtfs-get-mobilitydb/mobilitydb.csv --cache_key=${cache_key}) mobilitydb.csv

    ARG --required area
    COPY (+bbox/bbox.txt --area=${area}) bbox.txt
    ARG bbox=$(cat bbox.txt)
    RUN python /gtfs/filter_feeds.py --bbox="${bbox}" --gtfs-rt-service-alerts < mobilitydb.csv > gtfs_feeds.csv

    SAVE ARTIFACT gtfs_feeds.csv /gtfs_feeds.csv AS LOCAL ./data/${area}-${cache_key}.gtfs_feeds.csv

gtfs-compute-bbox:
    FROM rust
    ARG --required transit_feeds
    ARG --required cache_key

    RUN apt-get update \
        && apt-get install -y --no-install-recommends zstd \
        && rm -rf /var/lib/apt/lists/*

    COPY ./services/gtfs/gtfs_bbox .
    WORKDIR gtfs_bbox
    RUN cargo build --release

    COPY (+gtfs-build/gtfs.tar.zst --transit_feeds=${transit_feeds} --cache_key=${cache_key}) gtfs.tar.zst

    RUN mkdir gtfs_zips gtfs && \
        (cd gtfs_zips && \
            tar --zstd -xf ../gtfs.tar.zst && \
            ls *.zip | while read zip_file; do unzip -d ../gtfs/$(basename $zip_file .zip) $zip_file; done)

    RUN cargo run --release gtfs/* > bbox.txt

    SAVE ARTIFACT bbox.txt /bbox.txt

bbox:
    FROM debian:bullseye-slim
    ARG --required area
    COPY services/gtfs/bboxes.csv /gtfs/bboxes.csv
    # ensure `area` has an entry in bboxes.csv, otherwise you'll need to add one
    RUN test $(grep "${area}:" /gtfs/bboxes.csv | wc -l) -eq 1
    RUN grep "${area}:" /gtfs/bboxes.csv | cut -d':' -f2 | tee bbox.txt
    SAVE ARTIFACT bbox.txt /bbox.txt

gtfs-get-mobilitydb:
    FROM +gtfs-base
    ARG --required cache_key
    RUN curl 'https://storage.googleapis.com/storage/v1/b/mdb-csv/o/sources.csv?alt=media' > mobilitydb.csv
    SAVE ARTIFACT mobilitydb.csv mobilitydb.csv AS LOCAL "./data/mobilitydb-${cache_key}.csv"

save-otp-router-config:
    FROM +gtfs-base
    ARG --required transit_feeds
    ARG transit_zone=$(basename $transit_feeds | sed 's/.gtfs_feeds.csv//')

    COPY "${transit_feeds}" gtfs_feeds.csv
    COPY ./services/gtfs/filter_feeds.py /gtfs
    COPY ./services/gtfs/otp_router_config.py /gtfs
    WORKDIR /gtfs

    RUN ./otp_router_config.py < gtfs_feeds.csv > router-config.json

    SAVE ARTIFACT router-config.json /router-config.json AS LOCAL "./data/otp/${transit_zone}-router-config.json"

gtfs-build:
    FROM +gtfs-base
    ARG --required transit_feeds
    ARG --required cache_key

    RUN apt-get update \
        && apt-get install -y --no-install-recommends zstd \
        && rm -rf /var/lib/apt/lists/*

    COPY "${transit_feeds}" gtfs_feeds.csv
    COPY ./services/gtfs/download_gtfs_feeds.py ./

    # re-run when cache_key changes
    RUN touch "cache-buster-${cache_key}"

    RUN ./download_gtfs_feeds.py --output=downloads < gtfs_feeds.csv
    RUN cd downloads && ls *.zip | tar --zstd -cf /gtfs/gtfs.tar.zst --files-from -
    SAVE ARTIFACT /gtfs/gtfs.tar.zst /gtfs.tar.zst


##############################
# OpenTripPlanner
##############################

otp-base:
    # The version tag is ignored when sha256 is specified, but I'm leaving it
    # in to document which "release" our sha pins to.
    FROM opentripplanner/opentripplanner:2.3.0@sha256:630779e4b595462502b3813c1d6141da3e180d266d4a26371cc4ab6cb3390db0

    RUN mkdir /var/opentripplanner

otp-build:
    FROM +otp-base

    ARG --required area

    # Clip the mapping data area to the transit_feeds's bbox to save memory.
    #
    # This option is only relevant if you are configuring small transit
    # zones within an otherwise huge map. OTP reads all the map data into
    # memory, which can be very large.
    ARG clip_to_gtfs

    ARG --required transit_feeds
    ARG --required cache_key

    # Optional path to configuration that specifies non-default build options
    # See https://docs.opentripplanner.org/en/v2.2.0/BuildConfiguration
    ARG otp_build_config

    WORKDIR /var/opentripplanner

    RUN apt-get update \
        && apt-get install -y --no-install-recommends zstd \
        && rm -rf /var/lib/apt/lists/*

    IF [ -n "$otp_build_config" ]
        COPY "${otp_build_config}" /var/opentripplanner/build-config.json
    END

    IF [ -n "$clip_to_gtfs" ]
        COPY (+gtfs-compute-bbox/bbox.txt --transit_feeds=${transit_feeds} --cache_key=${cache_key}) bbox.txt
        ARG clip_bbox=$(cat bbox.txt)
        COPY (+extract/data.osm.pbf --area=${area} --clip_bbox=${clip_bbox}) /var/opentripplanner
    ELSE
        COPY (+extract/data.osm.pbf --area=${area}) /var/opentripplanner
    END

    COPY (+gtfs-build/gtfs.tar.zst --transit_feeds=${transit_feeds} --cache_key=${cache_key}) /var/opentripplanner
    RUN tar --zstd -xf gtfs.tar.zst

    RUN --entrypoint -- --build --save

    SAVE ARTIFACT /var/opentripplanner/graph.obj /graph.obj

otp-init-image:
    FROM +downloader-base
    COPY ./services/otp/init.sh /app/init.sh
    CMD ["/app/init.sh"]
    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/opentripplanner-init:${tag}
    END

otp-serve-image:
    FROM +otp-base

    EXPOSE 8000
    ENV PORT 8000

    # We add a layer of `sh -c` indirection in order to substitute in the PORT
    # env variable at runtime
    ENTRYPOINT ["sh", "-c"]
    CMD ["/docker-entrypoint.sh --load --port ${PORT}"]

    # used for healthcheck
    RUN apt-get update \
        && apt-get install -y --no-install-recommends netcat \
        && rm -rf /var/lib/apt/lists/*

    HEALTHCHECK --interval=5s --start-period=120s \
        CMD nc -z localhost ${PORT}

    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/opentripplanner:${tag}
    END

build-transitmux:
    FROM rust

    WORKDIR transitmux

    # This speeds up rebuilds of rust projectst by caching the prebuilt
    # dependencies in a separate docker layer. Without this, every change to
    # the source requires re-downloading and re-building all the project deps,
    # which takes a while.
    COPY ./services/transitmux/Cargo.toml .
    COPY ./services/transitmux/Cargo.lock .
    RUN mkdir src
    RUN echo 'fn main() { /* dummy main to get cargo to build deps */ }' > src/main.rs
    RUN cargo build --release
    RUN rm src/main.rs

    COPY ./services/transitmux .
    RUN cargo build --release
    SAVE ARTIFACT target/release/transitmux-server /transitmux-server

transitmux-serve-image:
    FROM debian:bullseye-slim

    RUN adduser --disabled-login transitmux --gecos ""
    USER transitmux

    WORKDIR /home/transitmux
    COPY +build-transitmux/transitmux-server transitmux-server

    EXPOSE 8000
    ENV RUST_LOG=info
    ENTRYPOINT ["/home/transitmux/transitmux-server"]
    CMD ["http://opentripplanner:8000/otp/routers"]

    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/transitmux:${tag}
    END

##############################
# Valhalla
##############################

valhalla-base-image:
    # The version tag is ignored when sha256 is specified, but I'm leaving it
    # in to document which "release" our sha pins to.
    FROM ghcr.io/gis-ops/docker-valhalla/valhalla:3.4.0@sha256:b6f20757c5a9d8bb432b53cb2923af36eb8908486d97fd1fdd114499a6d2a436

    USER root
    WORKDIR /tiles
    RUN chown valhalla /tiles
    USER valhalla

valhalla-build:
    FROM +valhalla-base-image

    RUN valhalla_build_config --mjolnir-tile-dir /tiles --mjolnir-timezone /tiles/timezones.sqlite --mjolnir-admin /tiles/admins.sqlite > valhalla.json
    RUN valhalla_build_timezones > /tiles/timezones.sqlite

    ARG --required area

    USER root
    RUN mkdir -p /data/osm
    COPY (+extract/data.osm.pbf --area=${area}) /data/osm/data.osm.pbf

    USER valhalla
    RUN valhalla_build_tiles -c valhalla.json /data/osm/data.osm.pbf

    SAVE ARTIFACT /tiles /tiles

valhalla-build-polylines:
    FROM +valhalla-build

    RUN valhalla_export_edges valhalla.json > /tiles/polylines.0sv

    SAVE ARTIFACT /tiles/polylines.0sv

valhalla-init-image:
    FROM +valhalla-base-image
    USER root

    RUN apt-get update \
        && apt-get install -y --no-install-recommends wget zstd \
        && rm -rf /var/lib/apt/lists/*

    USER valhalla
    COPY ./services/valhalla/init.sh /app/init.sh
    ENTRYPOINT ["/bin/bash"]
    USER root
    CMD ["/app/init.sh"]
    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/valhalla-init:${tag}
    END

valhalla-serve-image:
    FROM +valhalla-base-image
    ENTRYPOINT ["valhalla_service"]
    USER valhalla
    CMD ["/data/valhalla.json"]
    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/valhalla:${tag}
    END

##############################
# tileserver-gl-light
##############################

tileserver-build:
    FROM node:20-slim

    COPY ./services/tileserver/assets/build_glyphs.js \
        ./services/tileserver/assets/build_sprites.js \
        ./services/tileserver/assets/package.json \
        ./services/tileserver/assets/package-lock.json \
        ./services/tileserver/assets/*.ttf \
        /app/

    WORKDIR /app
    RUN npm install
    RUN mkdir -p /app/sprites/
    COPY ./services/tileserver/assets/sprites/*.svg /app/sprites/

    RUN mkdir /output
    RUN useradd -s /bin/bash fontnik
    RUN chown fontnik /output

    USER fontnik

    # Output fonts
    ENV FONTS_DIR=/output/fonts
    RUN mkdir "$FONTS_DIR"

    RUN mkdir "${FONTS_DIR}/Roboto Regular"
    RUN node build_glyphs Roboto-Regular.ttf "${FONTS_DIR}/Roboto Regular"

    RUN mkdir "${FONTS_DIR}/Roboto Medium"
    RUN node build_glyphs Roboto-Medium.ttf "${FONTS_DIR}/Roboto Medium"

    RUN mkdir "${FONTS_DIR}/Roboto Condensed Italic"
    RUN node build_glyphs Roboto-Condensed-Italic.ttf "${FONTS_DIR}/Roboto Condensed Italic"

    SAVE ARTIFACT "$FONTS_DIR" /fonts

    # Output sprite
    ENV SPRITE_DIR=/output/sprites
    RUN mkdir "$SPRITE_DIR"

    RUN node build_sprites "${SPRITE_DIR}/sprite" /app/sprites
    RUN node build_sprites --retina "${SPRITE_DIR}/sprite@2x" /app/sprites

    SAVE ARTIFACT "$SPRITE_DIR"  /sprites

tileserver-init-image:
    FROM +downloader-base

    COPY ./services/tileserver/init.sh /app/init.sh
    CMD ["/app/init.sh"]

    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/tileserver-init:${tag}
    END

tileserver-serve-image:
    FROM node:20-slim

    RUN npm install -g tileserver-gl-light

    USER root

    RUN apt-get update \
        && apt-get install -y gettext-base \
        && rm -rf /var/lib/apt/lists/*

    RUN mkdir -p /app/styles
    RUN chown -R node /app

    USER node

    COPY ./services/tileserver/styles/basic /app/styles/basic
    COPY (+tileserver-build/sprites) /app/sprites
    COPY (+tileserver-build/fonts) /app/fonts

    COPY ./services/tileserver/templates /templates/
    COPY ./services/tileserver/configure_run.sh /app/

    ENV HEADWAY_PUBLIC_URL=http://127.0.0.1:8080
    CMD ["/app/configure_run.sh"]
    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/tileserver:${tag}
    END

##############################
# Web
##############################

web-build:
    FROM node:20-slim
    RUN yarn global add @quasar/cli
    COPY ./services/frontend/www-app /www-app
    WORKDIR /www-app
    ARG branding
    IF [ ! -z ${branding} ]
        RUN sed -i "s/.*productName.*/  \"productName\": \"${branding}\",/" package.json
    END
    RUN yarn install && quasar build
    SAVE ARTIFACT /www-app/dist/spa /spa

web-init-image:
    FROM +downloader-base

    COPY ./services/frontend/init.sh ./services/frontend/generate_config.sh /app/
    ENV HEADWAY_SHARED_VOL=/data
    CMD ["/app/init.sh"]

    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/headway-init:${tag}
    END

web-serve-image:
    FROM nginx

    ARG branding
    COPY (+web-build/spa --branding=${branding}) /usr/share/nginx/html/

    COPY services/frontend/nginx.conf.template /etc/nginx/templates/nginx.conf.template

    ENV HEADWAY_PUBLIC_URL=http://127.0.0.1:8080
    ENV HEADWAY_SHARED_VOL=/data
    ENV HEADWAY_HTTP_PORT=8080
    ENV HEADWAY_RESOLVER=127.0.0.11
    ENV HEADWAY_TRANSITMUX_URL=http://transitmux:8000
    ENV HEADWAY_VALHALLA_URL=http://valhalla:8002
    ENV HEADWAY_TILESERVER_URL=http://tileserver:8000
    ENV HEADWAY_PELIAS_URL=http://pelias-api:8080
    # for escaping $ in nginx template
    ENV ESC=$
    ENV NGINX_ENVSUBST_OUTPUT_DIR=/etc/nginx

    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/headway:${tag}
    END

##############################
# Generic base images
##############################

downloader-base:
    FROM debian:bullseye-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends wget ca-certificates zstd \
        && rm -rf /var/lib/apt/lists/*
    RUN mkdir /data

save-base:
    FROM debian:bullseye-slim
    RUN apt-get update \
        && apt-get install -y --no-install-recommends zstd \
        && rm -rf /var/lib/apt/lists/*
