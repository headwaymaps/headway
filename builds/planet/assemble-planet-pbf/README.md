The planet.pbf we use for maps.earth is built from a few datasets (currently all from the daylight map distribution).

This directory is responsible for combining them all into a planet-wide pbf we can feed into Headway's earthly build process.

Perhaps it should be *part* of the earthly build process, but copying around large data files in Earthly entails building and copying around large docker layers which wastes both time and disk.

So for now anyway, this pbf assembly happens before and outside of the earthly build process.
