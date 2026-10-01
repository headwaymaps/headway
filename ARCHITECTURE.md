# Headway is...

...mostly a bunch of other projects stitched together.

A little more helpfully:

1. a bunch of backend services
2. a build system that compiles all the necessary data for those services (think map tiles, routing graphs, etc.)
3. a web frontend that uses those services
4. deployment configuration for running it all

## Backend services

- HTTP endpoint: nginx serves the frontend and reverse proxies to the other services
- map tiles
  - tile building: [planetiler](https://github.com/onthegomap/planetiler), using the [OpenMapTiles](https://openmaptiles.org) schema
  - tile server: [martin](https://martin.maplibre.org), which also serves the style, sprites, and fonts in [services/tileserver/assets](./services/tileserver/assets)
- geocoding (search): [pelias](https://pelias.io/), backed by Elasticsearch, placeholder, and libpostal
- routing
  - [travelmux](./services/travelmux) gives the frontend one routing API, forwarding each request to Valhalla or to the OpenTripPlanner instance covering the trip
  - bike, pedestrian, cars: [valhalla](https://github.com/valhalla/valhalla), planet-wide
  - transit: [OpenTripPlanner](https://www.opentripplanner.org/), one instance per transit zone. Within a zone, OTP also handles bike and walking directions.
- [transit-zoner](./TRANSIT_ZONER.md): an API over the index of GTFS feeds, for choosing the feeds that make up a transit zone

## Build system

[Our build system](./BUILD.md) is responsible for preparing service containers and the various data artifacts those services need. It's mostly built on [Dagger](https://dagger.io), with the Go code in [dagger](./dagger) usually invoked through scripts in [bin](./bin).

Data sources:

- [OpenStreetMap](https://www.openstreetmap.org) for tiles, routing, and search
- [Who's On First](https://whosonfirst.org) and [OpenAddresses](https://openaddresses.io) for search
- [Transitland Atlas](https://github.com/transitland/transitland-atlas) as the catalog of GTFS feeds. [gtfout](./services/gtfs/gtfout) measures and downloads those feeds for each transit zone.
- elevation tiles, fetched by `valhalla_build_elevation`, for OTP and travelmux

## Web frontend

Built on Quasar/Vue.js, with [MapLibre](https://maplibre.org) drawing the map. Find it in [services/frontend](./services/frontend/www-app).

## Deployment

Either docker compose ([docker-compose.yaml](./docker-compose.yaml), [docker-compose-with-transit.yaml](./docker-compose-with-transit.yaml)) or kubernetes templates in [k8s](./k8s), which we run on single node microk8s hosts.
