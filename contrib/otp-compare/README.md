# OTP compare

A development and review tool for seeing how route scoring changes affect a variety of routes. It
compares routes across the OTP and Valhalla builds in `docker-compose.yml`, with tiles and search from
maps.earth.

Each version runs its own OTP and travelmux. `compare.json` tells the frontend where to find each
travelmux and what to call it, and which of its routers to ask. headway's travelmux also fronts
both Valhallas.

`./inputs` holds the graph's inputs: an `*.osm.pbf`, an elevation `*.tif` and `*.gtfs.zip` feeds.
Use `OTP_COMPARE_INPUTS` to look elsewhere.

Valhalla serves the routing tiles `bin/build builds/Seattle valhalla` writes to `data/Seattle`.
Link or copy one to `./valhalla-tiles.tar.zst`, or point `VALHALLA_COMPARE_TILES` at it.
`VALHALLA_COMPARE_TAG` picks the `ghcr.io/headwaymaps/valhalla` images to run, `latest` by default.
Upstream Valhalla builds its own tiles from the `*.osm.pbf` in `./inputs`, at the commit on its `master`
that our fork branches from.

    bin/rebuild-and-restart

builds what changed, restarts it, and serves on `127.0.0.1:8080`, or `OTP_COMPARE_PORT`. Graphs persist in volumes, so after changing an
OTP commit or the inputs, `docker compose -p otp-compare down -v` rebuilds them.
