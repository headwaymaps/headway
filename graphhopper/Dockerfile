FROM openjdk:11-jre

RUN apt-get update

RUN apt-get install -y curl

WORKDIR /graphhopper

RUN curl -LJO https://github.com/graphhopper/graphhopper/releases/download/5.3/graphhopper-web-5.3.jar

RUN sh -c 'echo "98e379dbbc345d39e7e07101b48d106e81d0b09a1f67a23c18e6c45be54ff923  graphhopper-web-5.3.jar" | sha256sum -c'

COPY config.yaml /graphhopper

COPY startup.sh /graphhopper

ENTRYPOINT [ "/graphhopper/startup.sh" ]

EXPOSE 8989

CMD [ "-jar", "-Xmx6g", "-Xms6g", "graphhopper-web-5.3.jar", "serve", "/graphhopper/config.yaml" ]