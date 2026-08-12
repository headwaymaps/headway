# zone-builder-server

The interactive web app for defining transit zones from the GTFS feed-extents
index. It is a separate crate so the GTFS pipeline tools do not depend on
Actix.

Build the local Atlas clone and feed-extents index once:

```sh
bin/build-zone-builder-assets
```

Then start the server:

```sh
bin/start-zone-builder-server
```
