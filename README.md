# Headway

Headway is a maps stack in a box, allowing you to run `make Amsterdam` then `docker-compose up` to bring up a fully functional maps stack for the Amsterdam metro area including a frontend, basemap, geocoder and routing engine.

See BUILD.md for more information about the build process.

### Status

Headway is currently capable of showing a map, searching for points of interest and addresses within an OpenStreetMap extract and providing directions between any two places within that extract. Currently it only provides bicycling directions, but transit and walking directions are a work in progress.

The project is also missing:

- A kubernetes config for production use

### System Requirements

The machine used for generation of the data files needs to have at least 8GB of memory, potentially more for larger areas. We also recommend at least 50GB of free disk space, even if the OpenStreetMap extract for the area of interest is much smaller than that. Plan ahead to avoid disk pressure.

### License

Headway is available freely under the terms of the Apache License, verion 2.0. Please consider opening a PR for any enhancements or bugfixes!
