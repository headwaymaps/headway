FROM rust:bookworm AS build
WORKDIR /src
COPY . .
# cargo loads every workspace member, so the others need a target.
RUN for m in services/gtfs/transit-zoner services/gtfs/gtfout services/tileserver/maplibre-style; do \
      mkdir -p $m/src && touch $m/src/lib.rs; \
    done
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo install --locked --path services/travelmux --root /usr/local/installed

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates
COPY --from=build /usr/local/installed/bin/travelmux-server /usr/local/bin/travelmux-server
USER nobody
ENTRYPOINT ["/usr/local/bin/travelmux-server"]
