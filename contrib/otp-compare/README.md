# OTP compare

Compares routes across the OTP versions in `docker-compose.yml`, with tiles and search from
maps.earth.

`./inputs` holds the graph's inputs: an `*.osm.pbf`, an elevation `*.tif` and `*.gtfs.zip` feeds.
Use `OTP_COMPARE_INPUTS` to look elsewhere.

    docker compose up -d --build

serves on `127.0.0.1:8080`, or `OTP_COMPARE_PORT`. Graphs persist in volumes, so after changing an
OTP commit or the inputs, `docker compose down -v` rebuilds them.

`cloudflared.example.yml` publishes it.
