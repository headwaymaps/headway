FROM ubuntu:jammy

RUN apt-get update

RUN apt-get install -y tzdata

ENV TZ="America/New_York"

RUN apt-get install -y postgresql postgis curl openjdk-8-jre sudo

RUN mkdir /nominatim_data

COPY data.nominatim.tgz /nominatim_data/data.nominatim.tgz

WORKDIR /nominatim_data

RUN tar xvf data.nominatim.tgz

RUN mkdir /photon

WORKDIR /photon

RUN useradd -ms /bin/bash photon

RUN chown photon /photon

RUN curl -LJO https://github.com/komoot/photon/releases/download/0.3.5/photon-0.3.5.jar

RUN echo '6f438e905b020a70ec54de47b7c7f8b9c7921ef0ee84a3a9e0a6a6ebb7f74fa8  ./photon-0.3.5.jar' | sha256sum --check

COPY import_from_dump.sh /photon

USER root

RUN ["/bin/bash", "/photon/import_from_dump.sh"]

USER photon

CMD [ "java", "-jar", "photon-0.3.5.jar" ]
