#!/bin/bash
export HEADWAY_TRANSIT_ZONES HEADWAY_ENABLE_TRANSIT_ROUTING

# Optional so a partial build, or Compose stopping services, still works.
HEADWAY_TRANSIT_ZONES=$(bin/artifacts --optional otp-zones "$CONFIG_DIR")

if [ -n "$HEADWAY_TRANSIT_ZONES" ]; then
    HEADWAY_ENABLE_TRANSIT_ROUTING=1
else
    HEADWAY_ENABLE_TRANSIT_ROUTING=0
fi
