FROM node:22-slim AS build
ARG ROUTER_LINKS
WORKDIR /www-app
COPY services/frontend/www-app/ .
RUN yarn install --frozen-lockfile --ignore-scripts && yarn build
COPY services/frontend/generate_config.sh /generate_config.sh
RUN mkdir -p dist/spa/static && \
    HEADWAY_ENABLE_TRANSIT_ROUTING=1 \
    HEADWAY_BBOX="-122.462 47.394 -122.005 47.831" \
    HEADWAY_TRIP_CORPUS_URL=/corpus.json \
    HEADWAY_DEBUG_ENABLED=1 \
    HEADWAY_ROUTER_LINKS="$ROUTER_LINKS" \
    bash /generate_config.sh > dist/spa/static/headway-config.json

FROM nginx:1.29
COPY --from=build /www-app/dist/spa /usr/share/nginx/html
COPY contrib/otp-compare/corpus.json /usr/share/nginx/html/corpus.json
COPY contrib/otp-compare/nginx.conf /etc/nginx/nginx.conf
