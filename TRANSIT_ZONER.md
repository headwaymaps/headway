# Self-hosting the zone authoring tool

Transit zones are normally authored with the hosted tool at
<https://maps.earth/transit-zones>, and
[Adding transit routing](./BUILD.md#adding-transit-routing) covers the build
steps around it. Run your own copy when you want to pick from feeds the
published index doesn't carry, or to measure feeds with credentials of your own.

The tool is two halves: `transit-zoner`, which serves the feed data as an API,
and the `/transit-zones` page of the frontend app, which is the map you click on.

## 1. Build the feed-extents index

`transit-zoner` reads which feeds cover which area from a GeoPackage index. The
hosted tool and the `transit-zoner` container both use the copy published as
`gtfs/feed-extents.gpkg` in
[headway-data](https://github.com/headwaymaps/headway-data). Running locally,
you build your own.

A feed behind an API key can only be measured with that key in hand, so write
the credentials skeleton first and fill in any tokens you have:

```sh
bin/build-gtfs-index --dry-run --write-config-template gtfs-secrets.json
```

Then build the index. Every catalogued feed gets downloaded and measured, so
the first run is slow (~30m):

```sh
bin/build-gtfs-index
```

This writes `data/transit-zoner/feed-extents.gpkg`, cloning the Transitland
Atlas alongside it. Runs are incremental - a feed is skipped once it has been
measured successfully - so re-running retries past failures, which is how tokens
added to `gtfs-secrets.json` later make it into the index.

## 2. Serve the API

```sh
services/gtfs/transit-zoner/start-dev-server
```

It listens on `127.0.0.1:8420`, and tells you what to run if the index isn't
there yet.

## 3. Point the map at it

The page lives in the frontend app, whose dev server proxies `/transit-zoner`
to <https://maps.earth> by default. To use your local API instead, swap in the
commented-out alternative in `services/frontend/www-app/quasar.config.ts`:

```ts
'/transit-zoner': {
  changeOrigin: true,
  target: 'http://127.0.0.1:8420',
  rewrite: (path) => path.replace(/^\/transit-zoner/, ''),
},
```

Then run the frontend and open `/transit-zones` on the port it prints (9000 by
default):

```sh
cd services/frontend/www-app && yarn dev
```

Save what you build there as your zone's `zone.json`, and pick up from step 1 of
[Adding transit routing](./BUILD.md#adding-transit-routing).

## Publishing an index

The `transit-zoner` container never builds an index; it bakes in the one from
headway-data. To ship an updated index, commit it there as
`gtfs/feed-extents.gpkg` and rebuild the image.
