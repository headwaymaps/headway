FROM mediagis/nominatim:4.0

RUN apt-get update -y && apt-get install -y pbzip2

COPY restore_then_start.sh /app

ENTRYPOINT [ "/bin/bash" ]

CMD [ "restore_then_start.sh" ]
