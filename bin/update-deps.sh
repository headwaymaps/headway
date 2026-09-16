set -ex

# `dagger develop` regenerates dagger/internal, which isn't checked in but `go get` needs.
(cd services/frontend/www-app && yarn upgrade) \
    && (cd services/pelias/generate_config && yarn upgrade) \
    && (cd tests/browser && yarn upgrade) \
    && cargo update \
    && dagger develop \
    && (cd dagger && go get -u ./... && go mod tidy)

# NOTE: no tileserver step for now - martin is pinned dependency in dagger/main.go for now.
