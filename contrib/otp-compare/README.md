# Router compare

A development and review tool for seeing how routing changes affect a variety of routes. It compares
routes across the router builds in `docker-compose.yml`, with tiles and search from maps.earth.

One travelmux fronts every build, each by a slug. `compare.json` names the builds to compare by
slug and labels them, and the frontend plans with all of them in one request.

Each build makes its graph from `./inputs`: an `*.osm.pbf`, an elevation `*.tif` and `*.gtfs.zip`
feeds. Use `OTP_COMPARE_INPUTS` to look elsewhere.

    bin/rebuild-and-restart

builds what changed, restarts it, and serves on `127.0.0.1:8080`, or `OTP_COMPARE_PORT`. Graphs
persist in volumes, so after changing a build's commit or the inputs, `docker compose -p otp-compare
down -v` rebuilds them.
