# Router compare

A development and review tool for seeing how routing changes affect a variety of routes. It compares
routes across the router builds in `docker-compose.yml`. Tiles and search come from maps.earth.

`compare.json` labels the builds to compare by their travelmux slug.

Each build makes its graph from `./inputs`: an `*.osm.pbf`, an elevation `*.tif` and `*.gtfs.zip`
feeds. Use `ROUTE_COMPARE_INPUTS` to look elsewhere.

    bin/rebuild-and-restart

builds what changed, restarts it, and serves on `127.0.0.1:8080`, or `ROUTE_COMPARE_PORT`. Graphs
persist in volumes, so after changing a build's commit or the inputs, `docker compose -p otp-compare
down -v` rebuilds them.
