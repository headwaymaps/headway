FROM gisops/valhalla:latest

USER root

RUN apt-get update

RUN apt-get install -y valgrind

RUN mkdir /build_tiles

COPY build_tiles.sh /build_tiles

ENTRYPOINT [ "/bin/bash" ]

CMD [ "/build_tiles/build_tiles.sh" ]