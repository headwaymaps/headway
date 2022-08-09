FROM node:16

RUN npm install -g tileserver-gl-light

USER root

RUN apt-get update -y && apt-get install -y gettext-base

RUN mkdir -p /app/styles
RUN chown node /app
RUN chown node /app/styles
USER node

COPY configure_run.sh config.json.template /app/

ENTRYPOINT [ "/bin/bash" ]

CMD [ "/app/configure_run.sh" ]
