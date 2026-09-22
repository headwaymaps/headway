<template>
  <div
    class="marker"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <!-- Everything maplibre doesn't own, so it can be faded as a whole. -->
    <div class="contents" :class="{ faded }">
      <div class="pulse" :style="{ backgroundColor: vehicle.color }"></div>
      <div class="chip" :style="{ borderColor: vehicle.color }">
        {{ vehicle.emoji }}
      </div>
      <div
        v-if="vehicle.badge"
        class="badge"
        :style="{ borderColor: vehicle.color }"
      >
        {{ vehicle.badge }}
      </div>
    </div>
    <div v-if="hovered" class="tooltip">
      <div class="route">
        <span>{{ vehicle.emoji }}</span>
        <span>{{ vehicle.routeName }}</span>
        <span v-if="vehicle.labelFormatted" class="label">
          {{ vehicle.labelFormatted }}
        </span>
      </div>
      <div v-if="boardingStop()" class="boarding-stop">
        <span>{{ boardingStop() }}</span>
        <span v-if="vehicle.stopsAwayFormatted" class="stops-away">
          {{ vehicle.stopsAwayFormatted }}
        </span>
      </div>
      <div class="age">
        <i class="material-icons realtime">rss_feed</i>
        <!-- Rendered only while hovered, so the age is current when it's read. -->
        <span>{{ freshness() }}</span>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType, ref } from 'vue';
import TransitVehicle from 'src/models/TransitVehicle';

export type TransitVehicleMarkerProps = {
  vehicle: TransitVehicle;
  /// Milliseconds between this device's clock and the one that timed the position.
  clockOffsetMs: number;
  /// Set on a vehicle running a route the traveler hasn't picked.
  faded: boolean;
};

/// A transit vehicle's live position: the emoji for its kind, ringed in the route's color and
/// pulsing to say the position is live, badged with the route it runs.
export default defineComponent({
  name: 'TransitVehicleMarker',
  props: {
    vehicle: {
      type: Object as PropType<TransitVehicle>,
      required: true,
    },
    clockOffsetMs: {
      type: Number,
      required: true,
    },
    faded: {
      type: Boolean,
      required: true,
    },
  },
  setup(props) {
    const hovered = ref(false);
    return {
      hovered,
      freshness: () =>
        props.vehicle.freshnessFormatted(
          new Date(Date.now() + props.clockOffsetMs),
        ),
      boardingStop: () =>
        props.vehicle.boardingStopFormatted(
          new Date(Date.now() + props.clockOffsetMs),
        ),
    };
  },
});
</script>

<style lang="scss" scoped>
.marker {
  position: relative;
  width: 22px;
  height: 22px;
}

.contents {
  position: absolute;
  inset: 0;
}

// The fade lands here rather than on the marker itself: maplibre writes `opacity` inline on the
// element it was handed, and only while that element is on screen, so it is not ours to set.
.faded {
  opacity: 0.3;
}

// The emoji sits on a white chip so it stays legible over any basemap, with the
// route's color as the ring around it.
.chip {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: 2px solid;
  background: white;
  box-sizing: border-box;
  box-shadow: 0 0 3px rgba(0, 0, 0, 0.4);
  font-size: 12px;
  line-height: 1;
}

.pulse {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  animation: pulse 2s ease-out infinite;
}

@keyframes pulse {
  0% {
    transform: scale(1);
    opacity: 0.6;
  }
  100% {
    transform: scale(3);
    opacity: 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .pulse {
    animation: none;
    opacity: 0.3;
    transform: scale(2);
  }
}

// Hangs off the lower right of the chip. White with a colored border rather than
// colored with white text: GTFS route colors run light (King County Metro's is
// yellow), so white-on-color can't be relied on to stay readable.
.badge {
  position: absolute;
  top: 13px;
  left: 14px;
  padding: 0 3px;
  border: 1px solid;
  border-radius: 6px;
  background: white;
  color: #111;
  font-size: 9px;
  font-weight: 700;
  line-height: 11px;
  white-space: nowrap;
  box-shadow: 0 0 2px rgba(0, 0, 0, 0.35);
}

// Riders on a touch screen have no hover, and a marker this small is an awkward tap target, so
// the tooltip is a pointer affordance - the badge is what carries the route on its own.
.tooltip {
  position: absolute;
  bottom: 28px;
  left: 50%;
  transform: translateX(-50%);
  white-space: nowrap;
  padding: 4px 8px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.8);
  color: white;
  font-size: 12px;
  pointer-events: none;
}

.route {
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
}

// Reads as an aside to the route, which is the thing being identified.
.label {
  font-weight: 400;
  opacity: 0.75;
}

.realtime {
  font-size: 13px;
}

.boarding-stop {
  display: flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
}

// An aside to the countdown, which is what a waiting rider reads first.
.stops-away {
  font-weight: 400;
  opacity: 0.75;
}

.age {
  display: flex;
  align-items: center;
  gap: 4px;
  opacity: 0.8;
}
</style>
