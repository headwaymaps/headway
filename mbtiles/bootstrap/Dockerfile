FROM debian:bullseye-slim

RUN apt-get update -y

RUN apt-get install -y --no-install-recommends wget ca-certificates

COPY download_to_volume.sh /

CMD ["/download_to_volume.sh"]
