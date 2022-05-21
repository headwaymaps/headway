FROM ubuntu:jammy

RUN apt-get update

RUN apt-get install -y tzdata

ENV TZ="America/New_York"

RUN apt-get install -y postgresql

RUN mkdir /db

COPY *.pgsql.tgz /db

COPY deploy.sh /db

COPY load.sql /db

WORKDIR /db

CMD [ "/db/deploy.sh" ]