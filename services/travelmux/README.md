# travelmux is a routing service

It sits in front of both Valhalla and OpenTripPlanner, providing a uniform response.

OpenTripPlanner (OTP) is a multi-modal transportation planner. It handles
transit, cycling, walking, and driving directions (though we don't currently
use OTP driving directions).

Valhalla is supports driving, walking, and cycling. Relatively cheaply, you can
host a planet-wide instance of Valhalla to get directions anywhere on the
planet.


OTP is necessary for transportation routing, and, anecdotally, it provides
better cycling and walking directions.

However, OTP does not scale as well as Valhalla. It's not feasible to have a
single planet-wide instance, so instead, we have a cluster of smaller
deployments, giving a patchwork of coverage.

Travelmux takes care of forwarding your trip plan request to the appropriate
OTP instance.

A deployment could cover a town or metropolitan area. I've heard of OTP
instances as large as all of Europe, but I expect you will need quite a lot of
RAM. As an example, the Los Angeles OTP instance used for https://maps.earth
uses 3-4GB while mostly idle.

## Services

                     GET /travelmux/foo/bar
                               ↓
                   [ headway nginx frontend ]
                               ↓
    [ ******************** travelmux ******************** ]
      ↓                 ↓                ↓            ↓...
[ valhalla ]  [ OTP Los Angeles ]  [ OTP Puget ]  [ OTP ...]


## API versions

Each version is a directory under `src/api/`, mounted at its own path prefix
(`/v7/plan`, `/v7/directions`, `/v7/elevation`, ...). Versions are self-contained
rather than layered on each other, so an old one keeps behaving the way its
clients expect while a new one is free to change shape.

- **v7** is current. Its response types were designed around OTP's GraphQL API:
  times are RFC 3339 timestamps in the timezone of the graph that planned the
  trip, distances are always meters and durations always seconds (`distanceMeters`,
  `durationSeconds`), itineraries are returned directly rather than nested under
  `plan`, and the trip is fully described by our own types - there's no `_otp`
  raw passthrough. `preferredDistanceUnits` only chooses the units *instruction
  prose* is written in ("Continue for 2 miles."). Departure/arrival time is a
  single `dateTime`: an RFC 3339 instant, or a local wall clock time like
  `2024-06-13T14:30` interpreted in the graph's timezone.
- **v6** has the shape of OTP's old REST API - unix-millis timestamps, distances
  in whichever units the client asked for, and the upstream OTP and Valhalla
  responses echoed back under `_otp`/`_valhalla`. The iOS app still uses it; the
  web frontend has moved to v7. New clients should use v7.

## Development Setup

```
# expose valhalla and OTP ports
edit docker-compose-with-transit.yaml

```
diff --git a/docker-compose-with-transit.yaml b/docker-compose-with-transit.yaml
index dffe5bf..f06149a 100644
--- a/docker-compose-with-transit.yaml
+++ b/docker-compose-with-transit.yaml
@@ -47,8 +47,8 @@ services:
         condition: service_completed_successfully
     networks:
       - otp_backend
-    # ports:
-    #   - "9002:8000"
+    ports:
+      - "9002:8000"
   travelmux:
     image: ghcr.io/headwaymaps/travelmux:latest
     restart: always
@@ -88,8 +88,8 @@ services:
     depends_on:
       valhalla-init:
         condition: service_completed_successfully
-    # ports:
-    #   - "9001:8002"
+    ports:
+      - "9001:8002"
```

# consider blowing away any potentially stale containers
docker compose -f docker-compose-with-transit.yaml down --volumes
docker compose -f docker-compose-with-transit.yaml pull

# start services
docker compose -f docker-compose-with-transit.yaml up
```

start travelmux
```
cd services/travelmux
RUST_LOG=debug cargo run http://localhost:9001 http://localhost:9002

# or to rebuild on changes
RUST_LOG=debug cargo watch -- cargo run http://localhost:9001 http://localhost:9002
```

Edit quasar.config so that travelmux points at your local travelmux instance

```
diff --git a/services/frontend/www-app/quasar.config.js b/services/frontend/www-app/quasar.config.js
index 15aad52..42ecacb 100644
--- a/services/frontend/www-app/quasar.config.js
+++ b/services/frontend/www-app/quasar.config.js
@@ -113,10 +114,10 @@ module.exports = configure(function (/* ctx */) {
           // rewrite: (path) => path.replace(/^\/pelias/, ''),
         },
         '/travelmux': {
-          target: HEADWAY_HOST,
-          changeOrigin: true,
-          // target: 'http://0.0.0.0:8000',
-          // rewrite: (path) => path.replace(/^\/travelmux/, ''),
+          target: 'http://0.0.0.0:8000',
+          rewrite: (path) => path.replace(/^\/travelmux/, ''),
         },
       },
     },
```

Start the frontend dev server

```
cd services/frontend/www-app && yarn dev
```

At this point, you should be ready to go. Visit localhost:9000

## Talking to OTP

OTP is queried over its [GTFS GraphQL API](https://docs.opentripplanner.org/api/dev-2.x/graphql-gtfs/)
at `/otp/gtfs/v1`. The queries live in `src/otp/` as [cynic](https://cynic-rs.dev) structs, which
are checked against `schemas/otp.graphql` at compile time - so a field, argument, or nullability
that OTP doesn't agree with is a build error rather than a puzzling runtime response.

`schemas/otp.graphql` is the schema of the OTP version we deploy. When updating OTP, refresh it by
introspecting a running instance:

```
cargo install cynic-cli
cynic introspect http://localhost:9002/otp/gtfs/v1 -o services/travelmux/schemas/otp.graphql
```
