# Headway has basically two kinds of artifacts:
#
# 1. Map data, like tiles for rendering and routing and the geocoding database.
# 2. Containers that run services which use that map data.
#
# Theses things will change as the software evolves, and how these changes get
# deployed depends on the particular changes.
#
# This file attempts to track when breaking changes occur. Something like
# semantic versioning usually makes sense for this kind of thing in software
# libraries, but semver is less well suited for versioning an entire
# application deployment like headway. So, I don't claim that this is
# *strictly* "semantic" versioning.

# Tracks backwards incompatibilities between the built assets (map tiles, OTP
# graph, etc.)
#
# Major bumps to HEADWAY_DATA_TAG mean new versions of assets must be built and
# deployed before the corresponding containers can be deployed.
#
# A bump in the DATA tag pretty much always implies a bump in the CONTAINER
# tag, but not necessarily vice-versa.

export HEADWAY_DATA_TAG=0.9.0

# # Schema change Log
#
# ## DATA v0.9.0, CONTAINER v0.11.0
#
# BREAKING DATA/CONTAINER: Updated OTP to 2.5.0
#
# ## DATA v0.8.0, CONTAINER v0.10.0
#
# BREAKING DATA: pelias is now on es8
# BREAKING CONTAINER: travelmux added api v3, removed v1
#
# ## DATA v0.7.0, CONTAINER v0.9.0
#
# BREAKING CONTAINER: transitmux is now travelmux, and additionally fronts Valhalla, not just OTP
#
# ## DATA v0.7.0, CONTAINER v0.8.0
#
# BREAKING DATA: Updated Planetiler
#
# ## DATA v0.6.0, CONTAINER v0.7.0
#
# BREAKING DATA: Updated OTP and Valhalla
#
# ## DATA v0.5.0, CONTAINER v0.6.0
#
# BREAKING DATA: Updated OTP container/build, regenerated artifact
#
# ## DATA v0.4.0, CONTAINER v0.5.0
#
# BREAKING DATA: Updated OTP container/build, regenerated artifact
#
# ## DATA v0.3.0, CONTAINER v0.4.0
#
# BREAKING DATA: Updated OTP container/build, regenerated artifact
# BREAKING DATA: Updated Valhalla, regenerated artifact
#
# ## DATA v0.2.0, CONTAINER v0.3.0
#
# NEW CONTAINER: Added transitmux and opentriplanner-${AREA} k8s service
# BREAKING CONTAINER: removed singular opentriplanner service
#
# ## DATA v0.1.0, CONTAINER v0.2.0
#
# NEW DATA: zstd compression (maybe this should have been a breaking bump?)
#
# ## DATA v0.1.0, CONTAINER v0.1.0
#
# Lots of breaking changes happened before this, but 0.1.0 marks the beginning
# of trying to track them.
#
