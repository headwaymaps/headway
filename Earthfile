VERSION 0.8

##############################
# OSM extract
##############################
ARG --global is_planet_build = false

build:
    # The name of <area>.osm.pbf if you've downloaded a custom extract, or the
    # name of one of the pre-configured downloadable extracts available from
    # bbike.org
    ARG --required area

    # `countries` is used by whosonfirst dataset.
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
        BUILD +save-otp-graph --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=0
    END
    BUILD +save-valhalla --area=${area}
    BUILD +save-pelias --area=${area} --countries=${countries}
    BUILD +save-tileserver-terrain

save-pelias:
    FROM +save-base
    ARG --required area
    ARG countries
    BUILD +save-elasticsearch --area=${area} --countries=${countries}
    BUILD +save-placeholder --area=${area} --countries=${countries}
    BUILD +save-pelias-config --area=${area} --countries=${countries}

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

    COPY (+gtfs-build/gtfs --transit_feeds=${transit_feeds} --cache_key=${cache_key}) /gtfs
    RUN tar --use-compress-program="zstd -T0" -cf /gtfs.tar.zst -C /gtfs .
    # This isn't used at runtime, but it might be useful to archive the input
    SAVE ARTIFACT /gtfs.tar.zst AS LOCAL ./data/${area}-${output_prefix}-${cache_key}.gtfs.tar.zst

save-otp-zones:
    FROM +save-base
    ARG --required area
    # List of filenames. Each file corresponds to an OTP graph to output. Each row in each file references a GTFS feeds input to that OTP graph.
    ARG --required transit_zones
    ARG otp_build_config
    FOR transit_feeds IN $transit_zones
        BUILD +save-otp-graph --area=${area} --transit_feeds=${transit_feeds} --clip_to_gtfs=1 --otp_build_config=${otp_build_config}
        BUILD +save-otp-router-config --transit_feeds=${transit_feeds}
    END

save-otp-graph:
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

    COPY (+otp-build-graph/graph.obj --area=${area} \
                               --clip_to_gtfs=${clip_to_gtfs} \
                               --transit_feeds=${transit_feeds} \
                               --cache_key=${cache_key} \
                               --otp_build_config=${otp_build_config} \
    ) /graph.obj

    RUN zstd -T0 /graph.obj
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
    RUN tar --use-compress-program="zstd -T0" -cf /valhalla.tar.zst -C /valhalla .
    SAVE ARTIFACT /valhalla.tar.zst AS LOCAL ./data/${area}.valhalla.tar.zst

save-elasticsearch:
    FROM +save-base
    ARG --required area
    ARG countries
    COPY (+pelias-import/elasticsearch --area=${area} --countries=${countries}) /elasticsearch
    RUN tar --use-compress-program="zstd -T0" -cf /elasticsearch.tar.zst -C /elasticsearch .
    SAVE ARTIFACT /elasticsearch.tar.zst AS LOCAL ./data/${area}.elasticsearch.tar.zst

save-placeholder:
    FROM +save-base
    ARG --required area
    ARG countries
    COPY (+pelias-prepare-placeholder/placeholder --countries=${countries}) /placeholder
    RUN tar --use-compress-program="zstd -T0" -cf /placeholder.tar.zst -C /placeholder .
    SAVE ARTIFACT /placeholder.tar.zst AS LOCAL ./data/${area}.placeholder.tar.zst

save-pelias-config:
    FROM +save-base
    ARG --required area
    ARG countries
    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) /pelias.json
    SAVE ARTIFACT /pelias.json AS LOCAL ./data/${area}.pelias.json

save-tileserver-terrain:
    FROM +downloader-base
    ARG asset_root=https://github.com/headwaymaps/headway-data/raw/main/tiles/
    RUN wget -nv ${asset_root}/terrain.mbtiles
    SAVE ARTIFACT terrain.mbtiles AS LOCAL ./data/terrain.mbtiles
    RUN wget -nv ${asset_root}/landcover.mbtiles
    SAVE ARTIFACT landcover.mbtiles AS LOCAL ./data/landcover.mbtiles

images:
    FROM debian:bookworm-slim
    ARG tags="dev"
    ARG branding
    BUILD +travelmux-serve-image --tags=${tags}
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

# We use this both for import and for production pelias instances.
# But we might want to try a longer timeout for the import process?
pelias-config:
    FROM node:20-slim

    ARG --required area
    ARG countries

    COPY services/pelias/generate_config ./generate_config
    WORKDIR ./generate_config

    RUN yarn install && yarn build
    RUN bin/generate-pelias-config areas.csv "${area}" "${countries}" > pelias.json
    SAVE ARTIFACT pelias.json /pelias.json

pelias-import-base:
    FROM earthly/dind:alpine
    ARG --required area
    ARG countries

    WORKDIR /pelias-import

    RUN mkdir /data && chmod -R uga=rwX /data
    ENV DATA_DIR=/data

    COPY (+pelias-config/pelias.json --area=${area} --countries=${countries}) pelias.json
    COPY services/pelias/docker-compose-import.yaml compose.yaml
    COPY services/pelias/wait.sh ./tools/wait.sh
    COPY services/pelias/do-if-openaddresses-supported ./

    # Cache needed data in the base image so that multiple subsequent images don't need to
    # copy them individually.
    COPY (+extract/data.osm.pbf --area=${area}) /data/openstreetmap/data.osm.pbf
    WITH DOCKER --compose compose.yaml --service pelias_whosonfirst \
                                       --service pelias_openaddresses
        RUN docker-compose run -T 'pelias_whosonfirst' ./bin/download && \
            ./do-if-openaddresses-supported docker-compose run -T 'pelias_openaddresses' ./bin/download
    END

pelias-prepare-placeholder:
    ARG --required area
    ARG countries
    FROM +pelias-import-base --area=${area} --countries=${countries}

    WITH DOCKER --compose compose.yaml --service pelias_placeholder
        RUN docker-compose run -T 'pelias_placeholder' bash -c "./cmd/extract.sh && ./cmd/build.sh"
    END
    SAVE ARTIFACT /data/placeholder /placeholder

pelias-import:
    ARG --required area
    ARG countries
    FROM +pelias-import-base --area=${area} --countries=${countries}

    COPY (+valhalla-build-polylines/polylines.0sv --area=${area}) /data/polylines/extract.0sv

    RUN mkdir -p /data/elasticsearch && chmod 777 /data/elasticsearch

    WITH DOCKER --compose compose.yaml --service pelias_schema \
                                       --service pelias_openstreetmap \
                                       --service pelias_openaddresses \
                                       --service pelias_whosonfirst \
                                       --service pelias_polylines_import

        RUN docker-compose run -T 'pelias_schema' /tools/wait.sh && \
            docker-compose run -T 'pelias_schema' ./bin/create_index && \
            docker-compose run -T 'pelias_whosonfirst' ./bin/start && \
            ./do-if-openaddresses-supported docker-compose run -T 'pelias_openaddresses' ./bin/start && \
            docker-compose run -T 'pelias_openstreetmap' ./bin/start && \
            docker-compose run -T 'pelias_polylines_import' ./bin/start
    END

    SAVE ARTIFACT /data/elasticsearch /elasticsearch

##############################
# Planetiler
##############################

planetiler-build-mbtiles:
    FROM ghcr.io/onthegomap/planetiler:0.7.0

    RUN mkdir -p /data/sources
    RUN curl --no-progress-meter https://f000.backblazeb2.com/file/headway/sources.tar | tar -x --directory /data/sources

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

    COPY ./services/tilebuilder/percent-of-available-memory .

    IF [ "$is_planet_build" = "false" ]
      RUN --entrypoint -- \
          --osm_path=/data/data.osm.pbf \
          --force
    ELSE
      RUN --entrypoint -- \
          -Xmx$(./percent-of-available-memory 75) \
          `# return unused heap memory to the OS` \
          -XX:MaxHeapFreeRatio=40 \
          --osm_path=/data/data.osm.pbf \

          --bounds=planet \
          `# Store temporary node locations at fixed positions in a memory-mapped file` \
          --nodemap-type=array \
          --storage=mmap \
          --force
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
    FROM debian:bookworm-slim
    RUN --no-cache echo $(date +%Y-%m-%d) > todays_date
    SAVE ARTIFACT todays_date

# Get a list of all gtfs feeds that intersect with the given area
# Feeds are sourced from The Mobility Database Catalogs
# https://github.com/MobilityData/mobility-database-catalogs
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

gtfout:
    FROM rust:bookworm

    COPY ./services/gtfs/gtfout /gtfout
    WORKDIR /gtfout
    RUN cargo build --release

    SAVE ARTIFACT target/release/gtfs-bbox gtfs-bbox
    SAVE ARTIFACT target/release/assume-bikes-allowed assume-bikes-allowed

gtfs-compute-bbox:
    FROM debian:bookworm-slim

    ARG --required transit_feeds
    ARG --required cache_key

    COPY +gtfout/gtfs-bbox .

    RUN apt-get update \
        && apt-get install -y --no-install-recommends unzip \
        && rm -rf /var/lib/apt/lists/*

    COPY (+gtfs-build/gtfs --transit_feeds=${transit_feeds} --cache_key=${cache_key}) /gtfs_zips

    RUN mkdir gtfs && \
        (cd gtfs_zips && \
            ls *.zip | while read zip_file; do unzip -d ../gtfs/$(basename $zip_file .zip) $zip_file; done)

    RUN ./gtfs-bbox gtfs/* > bbox.txt

    SAVE ARTIFACT bbox.txt /bbox.txt

bbox:
    FROM debian:bookworm-slim
    ARG --required area
    COPY services/gtfs/bboxes.csv /gtfs/bboxes.csv
    # ensure `area` has an entry in bboxes.csv, otherwise you'll need to add one
    RUN test $(grep "${area}:" /gtfs/bboxes.csv | wc -l) -eq 1
    RUN grep "${area}:" /gtfs/bboxes.csv | cut -d':' -f2 | tee bbox.txt
    SAVE ARTIFACT bbox.txt /bbox.txt

gtfs-get-mobilitydb:
    FROM +downloader-base
    ARG --required cache_key
    RUN wget 'https://storage.googleapis.com/storage/v1/b/mdb-csv/o/sources.csv?alt=media' -O mobilitydb.csv
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
        && apt-get install -y --no-install-recommends zip \
        && rm -rf /var/lib/apt/lists/*

    COPY +gtfout/assume-bikes-allowed .

    COPY "${transit_feeds}" gtfs_feeds.csv
    COPY ./services/gtfs/download_gtfs_feeds.py ./

    # re-run when cache_key changes
    RUN touch "cache-buster-${cache_key}"

    RUN ./download_gtfs_feeds.py --output=downloads < gtfs_feeds.csv

    RUN mkdir unzipped && \
        (cd downloads && \
            ls *.zip | while read zip_file; do unzip -d ../unzipped/$(basename $zip_file .zip) $zip_file; done)

    RUN mkdir -p /output/gtfs && for gtfs in unzipped/*; do \
            ./assume-bikes-allowed  \
                < "${gtfs}/routes.txt" \
                > tmp-routes.txt \
            && mv tmp-routes.txt "${gtfs}/routes.txt" \
            && (cd "$gtfs" && zip -r "/output/gtfs/$(basename ${gtfs}).zip" .); \
        done

    SAVE ARTIFACT /output/gtfs /gtfs


##############################
# OpenTripPlanner
##############################

otp-base:
    FROM opentripplanner/opentripplanner:2.5.0

    RUN mkdir /var/opentripplanner

otp-build-graph:
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

    IF [ -n "$otp_build_config" ]
        COPY "${otp_build_config}" /var/opentripplanner/build-config.json
    END

    # Note: This bounds all directions to the extent of the transit feeds, so e.g. you can't get OTP
    # bike routing anywhere outside the bounds of the transit feeds. This should usually be fine, but it'd
    # be nice to handle the case where someone wants biking outside of their transit graph bbox.
    COPY (+gtfs-compute-bbox/bbox.txt --transit_feeds=${transit_feeds} --cache_key=${cache_key}) bbox.txt
    ARG gtfs_bbox=$(cat bbox.txt)

    IF [ -n "$clip_to_gtfs" ]
        COPY (+extract/data.osm.pbf --area=${area} --clip_bbox=${gtfs_bbox}) /var/opentripplanner
    ELSE
        COPY (+extract/data.osm.pbf --area=${area}) /var/opentripplanner
    END

    COPY (+elevation/elevation-tifs --bbox=${gtfs_bbox}) /var/opentripplanner

    COPY (+gtfs-build/gtfs --transit_feeds=${transit_feeds} --cache_key=${cache_key}) /var/opentripplanner

    RUN --entrypoint -- --build --save

    SAVE ARTIFACT /var/opentripplanner/graph.obj /graph.obj

download-elevation:
    FROM +valhalla-base-image

    # e.g. '-122.462 47.394 -122.005 47.831'
    # Note: this is the bbox format we use everywhere, but we need to convert it to the comma separated one that valhalla uses
    ARG --required bbox

    ARG valhalla_bbox=$(echo ${bbox} | sed 's/ /,/g')
    RUN valhalla_build_elevation --outdir elevation-hgts --from-bbox=${valhalla_bbox}

    SAVE ARTIFACT elevation-hgts

elevation:
    FROM debian:bookworm-slim
    ARG --required bbox

    RUN apt-get update \
        && apt-get install -y --no-install-recommends gdal-bin \
        && rm -rf /var/lib/apt/lists/*

    COPY services/otp/dem-hgt-to-tif .

    COPY (+download-elevation/elevation-hgts --bbox=${bbox}) elevation-hgts/

    RUN ./dem-hgt-to-tif elevation-hgts elevation-tifs

    SAVE ARTIFACT elevation-tifs

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

build-travelmux:
    FROM rust:bookworm

    WORKDIR travelmux

    # This speeds up rebuilds of rust projectst by caching the prebuilt
    # dependencies in a separate docker layer. Without this, every change to
    # the source requires re-downloading and re-building all the project deps,
    # which takes a while.
    COPY ./services/travelmux/Cargo.toml .
    COPY ./services/travelmux/Cargo.lock .
    RUN mkdir src
    RUN echo 'fn main() { /* dummy main to get cargo to build deps */ }' > src/main.rs
    RUN cargo build --release
    RUN rm src/main.rs

    COPY ./services/travelmux .
    RUN cargo build --release
    SAVE ARTIFACT target/release/travelmux-server /travelmux-server

travelmux-serve-image:
    FROM debian:bookworm-slim

    RUN apt-get update \
        && apt-get install -y --no-install-recommends libssl3 \
        && rm -rf /var/lib/apt/lists/*

    RUN adduser --disabled-login travelmux --gecos ""
    USER travelmux

    WORKDIR /home/travelmux
    COPY +build-travelmux/travelmux-server travelmux-server

    EXPOSE 8000
    ENV RUST_LOG=info
    ENTRYPOINT ["/home/travelmux/travelmux-server"]
    CMD ["http://valhalla:8002", "http://opentripplanner:8000/otp/routers"]

    ARG --required tags
    FOR tag IN ${tags}
        SAVE IMAGE --push ghcr.io/headwaymaps/travelmux:${tag}
    END

##############################
# Valhalla
##############################

valhalla-base-image:
    # Valhalla hasn't tagged a new version in over a year, so we track `latest`.
    FROM ghcr.io/gis-ops/docker-valhalla/valhalla

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
    ARG --required area
    FROM +valhalla-build --area=${area}

    RUN valhalla_export_edges -c valhalla.json > /tiles/polylines.0sv

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
    ENV HEADWAY_TRAVELMUX_URL=http://travelmux:8000
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
    FROM debian:bookworm-slim
    ENV TZ="America/New_York"
    RUN apt-get update \
        && apt-get install -y --no-install-recommends wget ca-certificates zstd \
        && rm -rf /var/lib/apt/lists/*
    RUN mkdir /data

save-base:
    FROM debian:bookworm-slim
    RUN apt-get update \
        && apt-get install -y --no-install-recommends zip zstd \
        && rm -rf /var/lib/apt/lists/*
