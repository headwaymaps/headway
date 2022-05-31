FROM gisops/valhalla:latest

USER root
WORKDIR /data/valhalla
RUN chown valhalla /data
RUN chown valhalla /data/valhalla
USER valhalla

COPY gen_hash.sh /app/gen_hash.sh

ENTRYPOINT ["/bin/bash"]

USER valhalla

CMD ["/app/gen_hash.sh"]