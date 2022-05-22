# Headway

Headway is a maps stack in a box. Upon completion, you will be able to run `make Amsterdam` then `docker-compose up` to bring up a fully functional maps stack for the Amsterdam area including a frontend, basemap, geocoder and a routing engine for driving, walking, bicycling and transit.

### Status

Headway is currently capable of taking a given city from the list in the Makefile and generating docker images preloaded with that city's data for:
* A photon geocoder
* An mbtileserver tile server
* A valhalla image

Currently missing:
* Frontend
* Reverse proxy config for docker-compose
* Kubernetes config for production use

### License

Headway is available freely under the terms of the AGPLv3. Please consider opening a PR for any enhancements! If you have commercial needs you're absolutely free to look through the build config of Headway to see which underlying software it uses for which parts of the build process. Most of the FOSS maps ecosystem is permissively licensed.