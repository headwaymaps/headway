# <p align=center>Headway</p>

<p align=center>
<img alt="GitHub Actions status badge" src="https://github.com/headwaymaps/headway/actions/workflows/checks.yml/badge.svg?branch=main"/>
<img alt="License badge" src="https://img.shields.io/github/license/headwaymaps/headway"/>
<img alt="GitHub last commit badge" src="https://img.shields.io/github/last-commit/headwaymaps/headway"/>
<img alt="GitHub commit activity badge" src="https://img.shields.io/github/commit-activity/m/headwaymaps/headway"/>
</p>

<p align=center>
<picture>
<source media="(prefers-color-scheme: dark)" srcset="assets/world_dark.svg?raw=true">
<img alt="World map image" src="assets/world_light.svg?raw=true">
</picture>
</p>

Headway is a maps stack in a box that makes it easy to take your location data into your own hands. With just a few commands you can bring up your own fully functional maps server. This includes a frontend, basemap, geocoder and routing engine. Choose one of the 200+ predefined cities or provide your own OpenStreetMap extract covering any area: from a neighborhood to the whole planet.

See [BUILD.md](./BUILD.md) for more information about the build process.

### Status

Headway is currently capable of showing a map, searching for points of interest and addresses within an OpenStreetMap extract and providing directions between any two places within that extract. Supported modes include driving, cycling and walking. Transit directions are a work-in-progress.

### System Requirements

Headway has been confirmed working on amd64 machines running Linux and macOS. The machine used for generation of the data files needs to have at least 8GB of memory, potentially more for larger areas. The requirements for running an instance of the stack are lower though. Expect to need around 4GB for a medium sized metro area. Additionally, you should expect to need 50GB-100GB of disk space during the build process.

### License

Headway is available freely under the terms of the Apache License, version 2.0. Please consider opening a PR for any enhancements or bugfixes!
