# Headway is:

1. a bunch of other projects stitched together
2. a consolidated build system for those projects
3. a web frontend

## Components

- web frontend
  - custom quasar/vue.js app in [services/frontend](https://github.com/headwaymaps/headway/tree/main/services/frontend)
- build system
  -  Earthly [read more](https://github.com/headwaymaps/headway/blob/main/BUILD.md)
- map tiles
  - tile building: [planetiler](https://github.com/onthegomap/planetiler)
  - tile server: https://www.npmjs.com/package/tileserver-gl-light
- geocoding: 
  - [pelias](https://pelias.io/)
- routing
  - bike, pedestrian, cars: [valhalla](https://github.com/valhalla/valhalla)
  - transit: [OpenTripPlanner](http://www.opentripplanner.org/)

