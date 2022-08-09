FROM debian:bullseye-slim as bootstrapper

RUN apt-get update -y && apt-get install -y --no-install-recommends wget ca-certificates

COPY download_mirrored_data.sh /

RUN /download_mirrored_data.sh

FROM debian:bullseye-slim

RUN apt-get update -y && apt-get install -y --no-install-recommends curl ca-certificates openjdk-17-jre-headless

WORKDIR /planetiler

ARG PLANETILER_VERSION=v0.4.0
ARG PLANETILER_HASH=2ec77024662909fc858a6aae16973f3f15a26429ccd88f06d5998ab4241108e0

ARG JAVA_MEM_ARGS=-Xmx8G

RUN curl -L -o planetiler.jar https://github.com/onthegomap/planetiler/releases/download/${PLANETILER_VERSION}/planetiler.jar

RUN bash -c 'echo "${PLANETILER_HASH}  planetiler.jar" | sha256sum --check'

COPY --from=bootstrapper /data/lake_centerline.shp.zip /data/natural_earth_vector.sqlite.zip /data/water-polygons-split-3857.zip /data/sources/

COPY data.osm.pbf /data/data.osm.pbf

WORKDIR /

RUN java ${JAVA_MEM_ARGS} -jar /planetiler/planetiler.jar --force osm_path=/data/data.osm.pbf
