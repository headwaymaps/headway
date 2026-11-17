FROM node:22-slim AS build
WORKDIR /www-app
COPY services/frontend/www-app/ .
RUN yarn install --frozen-lockfile --ignore-scripts && HEADWAY_COMPARE_ROUTES=1 yarn build
COPY services/frontend/generate_config.sh /generate_config.sh
RUN mkdir -p dist/spa/static && \
    HEADWAY_ENABLE_TRANSIT_ROUTING=1 \
    HEADWAY_BBOX="-122.462 47.394 -122.005 47.831" \
    bash /generate_config.sh > dist/spa/static/headway-config.json

FROM nginx:1.29
COPY --from=build /www-app/dist/spa /usr/share/nginx/html
COPY contrib/compare-routes/corpus.json /usr/share/nginx/html/corpus.json
COPY contrib/compare-routes/compare.json /usr/share/nginx/html/compare.json
COPY contrib/compare-routes/nginx.conf /etc/nginx/nginx.conf
