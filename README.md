# Headway

![GitHub last commit](https://img.shields.io/github/last-commit/ellenhp/headway)
![GitHub commit activity](https://img.shields.io/github/commit-activity/m/ellenhp/headway)
![GitHub](https://img.shields.io/github/license/ellenhp/headway)

Headway is a maps stack in a box that makes it easy to take your location data into your own hands. For example, `make Amsterdam` then `docker-compose up` will bring up a fully functional maps web app for the Amsterdam metro area. This includes a frontend, basemap, geocoder and routing engine. Over 200 different cities are currently supported.

See [BUILD.md](./BUILD.md) for more information about the build process.

### Status

Headway is currently capable of showing a map, searching for points of interest and addresses within an OpenStreetMap extract and providing directions between any two places within that extract. Currently it is capable of providing directions for driving, cycling and walking. Transit directions are a work-in-progress.

The project is missing a kubernetes config for production use. Contributions for productionization are very welcome! Please open an issue to discuss first though.

### System Requirements

Headway has been confirmed working on amd64 machines running Linux and macOS. The machine used for generation of the data files needs to have at least 8GB of memory, potentially more for larger areas. The requirements for running an instance of the stack are lower though. Expect to need around 4GB for a medium sized metro area. The requirements can be reduced further by the omission of transit routing. We also recommend at least 50GB of free disk space, even if the OpenStreetMap extract for the area of interest is much smaller than that. Plan ahead to avoid disk pressure.

### License

Headway is available freely under the terms of the Apache License, verion 2.0. Please consider opening a PR for any enhancements or bugfixes!
