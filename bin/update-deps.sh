set -ex

(cd services/frontend/www-app && yarn upgrade) \
    && (cd services/pelias/generate_config && yarn upgrade) \
    && (cd services/travelmux && cargo update) \
    && (cd services/gtfs/gtfout && cargo update) \
    && (cd dagger && go get -u ./... && go mod tidy)

# NOTE: no tileserver step for now - martin is pinned dependency in dagger/main.go for now.
