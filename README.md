# <p align=center>Headway</p>

<p align=center>
<img alt="GitHub Actions status badge" src="https://github.com/headwaymaps/headway/actions/workflows/main.yml/badge.svg?branch=main"/>
<img alt="License badge" src="https://img.shields.io/github/license/headwaymaps/headway"/>
<img alt="GitHub last commit badge" src="https://img.shields.io/github/last-commit/headwaymaps/headway"/>
<img alt="GitHub commit activity badge" src="https://img.shields.io/github/commit-activity/m/headwaymaps/headway"/>
</p>

<p align=center>
<img alt="World map image" src="assets/world.svg?raw=true">
</p>

Headway is a maps stack in a box that makes it easy to take your location data into your own hands. With just a few commands you can bring up your own fully functional maps server. This includes a frontend, basemap, geocoder and routing engine. Over 200 different cities are currently supported.

See [BUILD.md](./BUILD.md) for more information about the build process.

### Status

Headway is currently capable of showing a map, searching for points of interest and addresses within an OpenStreetMap extract and providing directions between any two places within that extract. Supported modes include driving, cycling and walking. Transit directions are a work-in-progress.

### System Requirements

Headway has been confirmed working on amd64 machines running Linux and macOS. The machine used for generation of the data files needs to have at least 8GB of memory, potentially more for larger areas. The requirements for running an instance of the stack are lower though. Expect to need around 4GB for a medium sized metro area. The requirements can be reduced further by the omission of transit routing. We also recommend at least 50GB of free disk space, even if the OpenStreetMap extract for the area of interest is much smaller than that. Plan ahead to avoid disk pressure.

For planet builds, expect to need 64GB of RAM. Hosting a planet-scale instance requires about 64GB of RAM as well. Earthly makes many needless copies of data in its cache, so unless you have ~10TB of NVMe storage you will need to perform planet builds on a system with either VDO enabled or on a filesystem that supports deduplication natively like ZFS. With deduplication enabled the disk space requirement for a full-planet build is fairly low, expect to need around 1TB of fast storage. You'll also need to disable BuildKit parallelism in earthly and set `cache_size_mb` to 10000000 to trick it into using more disk space than you technically have available. I've observed deduplication ratios as high as 15x, with an average of 8-10x for full-planet work.

### License

Headway is available freely under the terms of the Apache License, verion 2.0. Please consider opening a PR for any enhancements or bugfixes!
