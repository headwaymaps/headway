FROM ubuntu:focal

RUN apt-get update -y

RUN apt-get install -y wget

COPY download_to_volume.sh /

CMD ["/download_to_volume.sh"]