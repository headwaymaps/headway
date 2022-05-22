# Copyright (C) 2022 Ellen Poe

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.

# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see https://www.gnu.org/licenses/.

FROM gisops/valhalla:latest

USER root

RUN apt-get update

RUN apt-get install -y valgrind

RUN mkdir /build_tiles

RUN chown valhalla /build_tiles

COPY build_tiles.sh /build_tiles

ENTRYPOINT [ "/bin/bash" ]

CMD [ "/build_tiles/build_tiles.sh" ]