# Headway

Headway is a maps stack in a box. Upon completion, you will be able to run `make Amsterdam` then `docker-compose up` to bring up a fully functional maps stack for the Amsterdam metro area including a frontend, basemap, geocoder and a routing engine for driving, walking, bicycling and transit.

### Status

Headway is currently capable of taking a given city from the list in the Makefile and generating docker images preloaded with that city's data for:

- A nominatim geocoder
- A photon geocoder (used as an auto-complete service for nominatim)
- A tileserver-gl tile server
- A graphhopper image

The frontend is a work in progress. It is currently capable of:

- Showing base map tiles
- Letting you search for a place or an address (with autocomplete)
- Routing between any two POIs, including dropped pins.
- Providing a scrubbable timeline of a cycling route.

It is not capable of:

- Open-ended search (show all matches on the map or in a list)
- Clicking on a POI on the base map (technically the base map doesn't have any POIs because the tilserver-gl style isn't customized yet)

The project is also missing:

- A way to download GTFS feeds (maybe from [here](https://database.mobilitydata.org/)?)
- Kubernetes config for production use

### Architecture

Headway is designed to make it as easy as possible to bring up a full maps stack for a medium-sized metro area, so it generally eschews more complicated and scalable technologies and tries to do the simplest thing. For example, if you run `make Denver` you will end up with several enormous docker images pre-loaded with data for Denver. This is generally not a good way to do things, but it greatly simplifies zero-downtime data updates because you can just deploy a new set of containers instead of trying to update the volume that all the data lives on without causing issues to running containers.

### System Requirements

Processing GIS data is resource intensive, even when you only care about a single metro area. The machine used for generation of Docker images needs to be reasonably well equipped. The main requirement is 8GB of memory, but you should also have plenty of free disk space. Even if the OpenStreetMap extract for your metro area is only a few hundred megabytes, the search index for the geocoder will be many times that, and Headway also needs to create basemap tiles and routing tiles since the raw .osm.pbf file is too compact a representation to be useful for running a server. Basemap tile generation also requires Headway to download a map of all of the water on the planet, which takes up about a gigabyte of storage. You can also expect intermediate data files and a few superfluous copies to sit around on your hard drive taking up space. Estimates are hard to give because the size of the OpenStreetMap database is constantly increasing but it would be inadvisable to run e.g. `make Paris` unless you're okay with Headway creating about 50GB of files. Plan ahead to avoid disk pressure.

### License

Headway is available freely under the terms of the Apache License, verion 2.0. Please consider opening a PR for any enhancements or bugfixes!
