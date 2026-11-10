# OTP compare

A development and review tool for seeing how route scoring changes affect a variety of routes. It
compares routes across the OTP versions in `docker-compose.yml`, with tiles and search from
maps.earth.

Each version runs its own OTP and travelmux. `compare.json` tells the frontend where to find each
travelmux and what to call it.

`./inputs` holds the graph's inputs: an `*.osm.pbf`, an elevation `*.tif` and `*.gtfs.zip` feeds.
Use `OTP_COMPARE_INPUTS` to look elsewhere.

    bin/rebuild-and-restart

builds what changed, restarts it, and serves on `127.0.0.1:8080`, or `OTP_COMPARE_PORT`. Graphs persist in volumes, so after changing an
OTP commit or the inputs, `docker compose -p otp-compare down -v` rebuilds them.
