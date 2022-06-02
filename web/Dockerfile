FROM node:16-slim as builder

RUN yarn global add @quasar/cli

COPY frontend /frontend

WORKDIR /frontend

RUN yarn install \
  && yarn build

FROM nginx

COPY init.sh bboxes.csv /frontend/

COPY --from=builder /frontend/dist/spa/ /usr/share/nginx/html/

COPY nginx.conf.template /etc/nginx/templates/nginx.conf.template

ENV HEADWAY_PUBLIC_URL=http://127.0.0.1:8080
ENV HEADWAY_BBOX_PATH=/frontend/bbox.txt
ENV HEADWAY_SHARED_VOL=/data
ENV HEADWAY_HTTP_PORT=8080
ENV HEADWAY_RESOLVER=127.0.0.11
ENV HEADWAY_OTP_URL=http://otp:8080
ENV HEADWAY_VALHALLA_URL=http://valhalla:8002
ENV HEADWAY_PHOTON_URL=http://photon:2322
ENV HEADWAY_TILESERVER_URL=http://mbtileserver:8000
ENV HEADWAY_NOMINATIM_URL=http://nominatim:8080
# for escaping $ in nginx template
ENV ESC=$
ENV NGINX_ENVSUBST_OUTPUT_DIR=/etc/nginx
ENTRYPOINT ["/frontend/init.sh"]
