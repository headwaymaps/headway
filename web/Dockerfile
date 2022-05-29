FROM node:16-slim as builder

RUN yarn global add @quasar/cli

COPY frontend /frontend

WORKDIR /frontend

RUN yarn install \
  && quasar build

FROM nginx

COPY --from=builder /frontend/dist/spa/ /usr/share/nginx/html/

COPY nginx.conf .base_url /etc/nginx/

COPY bbox.txt /usr/share/nginx/html/bbox.txt

RUN bash -c 'sed -i "s^__HEADWAY_BASE_URL__^$(</etc/nginx/.base_url)^" /etc/nginx/nginx.conf'
