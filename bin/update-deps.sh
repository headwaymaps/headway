set -ex

(cd services/frontend/www-app && yarn upgrade) \
    && (cd services/pelias/generate_config && yarn upgrade) \
    && (cd services/travelmux && cargo update) \
    && (cd services/gtfs/gtfout && cargo update) \
    && (cd services/tileserver && yarn upgrade) \
    && (cd dagger && go get -u ./... && go mod tidy)
