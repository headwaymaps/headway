FROM gisops/valhalla:latest

COPY tiles.tar /custom_files/valhalla_tiles.tar

USER root

RUN md5sum /custom_files/valhalla_tiles.tar > /custom_files/file_hashes.txt

USER valhalla