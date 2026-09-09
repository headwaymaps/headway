# transit-zoner

The API over the GTFS feed-extents index: which measured feeds touch an area,
and the `zone.json` document for a chosen set of them. It is a separate crate
so the GTFS pipeline tools do not depend on Actix.

The UI that reads it is the `/transit-zones` page of the frontend app
(`services/frontend/www-app`), so drawing a zone reuses the basemap and the
geocoder rather than shipping a second map application.

Start the server, which builds the local index when needed:

```sh
services/gtfs/transit-zoner/start-dev-server
```

## Endpoints

All paths are under `/api`, and are proxied at `/transit-zoner/api/` by the
frontend's nginx so the page can call them same-origin.

- `GET /api/feeds-by-bbox?bbox=min_lon,min_lat,max_lon,max_lat` — the feeds
  whose measured extents intersect the box, best match first.
- `GET /api/feeds/{ids}` — the same summaries for feeds named by id, for
  reopening a zone file whose feeds may no longer intersect anything.
- `POST /api/zone` — `{"bbox": "...", "feed_ids": [...]}` in, the zone document
  out. Assembling it here rather than in the page keeps the format the build
  reads in one place; `gtfout::zone` verifies the build reads what this writes.

## Working on the page

Run this server and the frontend's dev server together, and point the latter's
proxy at `http://127.0.0.1:8420` for `/transit-zoner/api`.
