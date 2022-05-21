# Headway

Headway is a maps stack in a box. Upon completion, you will be able to run `make Amsterdam` then `docker-compose up` to bring up a fully functional maps stack for the Amsterdam area including a frontend, basemap, geocoder and a routing engine for driving, walking, bicycling and transit.

### Status

Headway is currently capable of taking a given city from the list in the Makefile, and generating docker images preloaded with that city's data for a Photon geocoder and an mbtileserver tile server.

### License

Headway is available freely under the terms of the AGPL. Please consider opening a PR for any enhancements! If you have commercial needs you're absolutely free to look through the build config of Headway to see which underlying software it uses for which parts of the build process. Most of the FOSS maps ecoystem is permissively licensed.

Copyright (C) 2022 Ellen Poe

This program is free software: you can redistribute it and/or modify \
it under the terms of the GNU Affero General Public License as published by \
the Free Software Foundation, either version 3 of the License, or \
(at your option) any later version.

This program is distributed in the hope that it will be useful, \
but WITHOUT ANY WARRANTY; without even the implied warranty of \
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the \
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License \
along with this program.  If not, see <https://www.gnu.org/licenses/>.
