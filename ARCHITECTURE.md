# Headway is...

...mostly a bunch of other projects stitched together.

A little more helpfully:

1. a bunch of backend services
2. a build system that compiles all the necessary data for those services (think map tiles, routing graphs, etc.)
3. a web frontend that uses those services

## Backend services

- HTTP endpoint: nginx reverse proxies to the other services
- map tiles
  - tile building: [planetiler](https://github.com/onthegomap/planetiler)
  - tile server: https://www.npmjs.com/package/tileserver-gl-light
- geocoding (search):
  - [pelias](https://pelias.io/)
- routing
  - bike, pedestrian, cars: [valhalla](https://github.com/valhalla/valhalla)
  - transit: [OpenTripPlanner](http://www.opentripplanner.org/)

## Build system

[Our build system](https://github.com/headwaymaps/headway/blob/main/BUILD.md) is responsible for prepare service containers and the various data artifacts those services need. It's mostly built on [Dagger](https://dagger.io).

## Web frontend

Built on quasar/vue.js. Find it in [services/frontend](https://github.com/headwaymaps/headway/tree/main/services/frontend/www-app).
