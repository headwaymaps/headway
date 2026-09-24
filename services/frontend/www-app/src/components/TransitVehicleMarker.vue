<template>
  <div
    class="marker"
    @mouseenter="popover.pointerEntered()"
    @mouseleave="popover.pointerLeft()"
    @click.stop="$emit('pin')"
  >
    <!-- Everything maplibre doesn't own, so it can be faded as a whole. -->
    <div class="contents" :class="{ faded }">
      <div class="pulse" :style="{ backgroundColor: vehicle.color }"></div>
      <div
        class="chip"
        :class="{ pinned: popover.isPinned }"
        :style="{ borderColor: vehicle.color }"
      >
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
    <div v-if="popover.isOpen" class="tooltip">
      <div class="route">
        <span>{{ vehicle.emoji }}</span>
        <span>{{ vehicle.routeName }}</span>
        <span v-if="vehicle.labelFormatted" class="label">
          {{ vehicle.labelFormatted }}
        </span>
      </div>
      <div v-if="boardingStop" class="boarding-stop">
        <span>{{ boardingStop.text }}</span>
        <span v-if="boardingStop.countdown" class="countdown">
          {{ boardingStop.countdown.value
          }}<span class="unit">{{ boardingStop.countdown.unit }}</span>
          <i class="material-icons realtime">rss_feed</i>
        </span>
      </div>
      <div class="age">
        <span>{{ freshness }}</span>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  computed,
  defineComponent,
  onUnmounted,
  PropType,
  ref,
  watch,
} from 'vue';
import TransitVehicle from 'src/models/TransitVehicle';
import VehiclePopover from 'src/models/VehiclePopover';

/// How often the open tooltip re-reads the clock. The vehicle keeps moving between polls, so the
/// countdown, the stop count and the age of the position all have to come down with it.
const TICK_MS = 1000;

export type TransitVehicleMarkerProps = {
  vehicle: TransitVehicle;
  /// Milliseconds between this device's clock and the one that timed the position.
  clockOffsetMs: number;
  /// Set on a vehicle running a route the traveler hasn't picked.
  faded: boolean;
  /// Set on the one vehicle whose popover the traveler has clicked open.
  pinned: boolean;
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
    pinned: {
      type: Boolean,
      required: true,
    },
  },
  emits: ['pin'],
  setup(props) {
    const popover = ref(new VehiclePopover());
    const tick = ref(Date.now());

    // Only while the tooltip is open: nothing else on the marker reads the clock, and a map can
    // carry dozens of these.
    let ticker: ReturnType<typeof setInterval> | undefined;
    const stopTicking = () => {
      clearInterval(ticker);
      ticker = undefined;
    };
    watch(
      () => popover.value.isOpen,
      (isOpen) => {
        stopTicking();
        tick.value = Date.now();
        if (isOpen) {
          ticker = setInterval(() => (tick.value = Date.now()), TICK_MS);
        }
      },
    );
    onUnmounted(stopTicking);

    watch(
      () => props.pinned,
      (pinned) => popover.value.setPinned(pinned),
    );

    const now = computed(() => new Date(tick.value + props.clockOffsetMs));
    return {
      popover,
      freshness: computed(() => props.vehicle.freshnessFormatted(now.value)),
      boardingStop: computed(() => props.vehicle.boardingStopRow(now.value)),
    };
  },
});
</script>

<style lang="scss" scoped>
.marker {
  position: relative;
  cursor: pointer;
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

// A thicker ring on the vehicle whose popover is being read, so it stays findable among its
// neighbours once the pointer has left it. The chip grows by what the ring gains, so the ring
// thickens outwards and the emoji inside keeps its size.
.chip.pinned {
  inset: -2px;
  border-width: 4px;
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

// Reads as an aside to the route, which is the thing being identified, and sits on the trailing
// edge above the wait - the same column the iOS marker puts it in.
.label {
  margin-left: auto;
  font-weight: 400;
  opacity: 0.75;
}

// Marks the countdown as coming off the live feed, where the age line below only dates it.
.realtime {
  margin-left: 2px;
  font-size: 12px;
  vertical-align: -1px;
}

.boarding-stop {
  display: flex;
  align-items: baseline;
  gap: 12px;
  font-weight: 600;
}

// Pushed to the trailing edge, so a column of vehicles lines its waits up.
.countdown {
  margin-left: auto;
  white-space: nowrap;
}

.unit {
  margin-left: 1px;
  font-size: 9px;
  font-weight: 400;
  opacity: 0.75;
}

.age {
  opacity: 0.8;
}
</style>
