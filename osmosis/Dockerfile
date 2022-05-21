FROM openjdk:latest

RUN mkdir /osmosis

RUN useradd -ms /bin/bash osmosis

WORKDIR /osmosis

RUN chown osmosis /osmosis

USER osmosis

RUN curl -LJO "https://github.com/openstreetmap/osmosis/releases/download/0.48.3/osmosis-0.48.3.tgz"

RUN echo 'b24c601578ea4cb0ca88302be6768fd0602bde86c254a0e0b90513581dba67ff  osmosis-0.48.3.tgz' | sha256sum --check

RUN tar -xvf osmosis-0.48.3.tgz -C .

CMD ["osmosis"]