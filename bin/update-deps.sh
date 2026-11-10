set -ex

# `dagger develop` regenerates dagger/internal, which isn't checked in but `go get` needs.
(cd services/frontend/www-app && yarn upgrade) \
    && (cd services/pelias/generate_config && yarn upgrade) \
    && (cd tests/browser && yarn upgrade) \
    && cargo update \
    && dagger develop \
    && (cd dagger && go get -u ./... && go mod tidy)

# NOTE: `go get -u` pulls go.opentelemetry.io/otel to v1.47+, which breaks the
# dagger build. Downgrade the otel modules back to v1.46.0 (otel/log and
# otel/sdk/log to v0.22.0) until github.com/dagger/otel-go supports the stable
# otel/log v1 API.

# NOTE: no tileserver step for now - martin is pinned dependency in dagger/main.go for now.
