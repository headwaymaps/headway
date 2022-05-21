FROM ubuntu:jammy

RUN mkdir /db

COPY *.pgsql.tgz /db
COPY deploy.sh /db

CMD [ "/db/deploy.sh" ]