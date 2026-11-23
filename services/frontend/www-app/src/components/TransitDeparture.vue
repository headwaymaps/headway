<template>
  <div
    v-if="formattedDurationUntilStart() !== undefined"
    class="transit-departure"
  >
    <span class="real-time-departure-time">
      {{ formattedDurationUntilStart() }}&nbsp;
    </span>
    <i18n-t
      v-if="firstTransitLegDepartureLocation()"
      keypath="departs_at_$location"
      tag="span"
      scope="global"
      class="real-time-departure-location"
    >
      <template #location>
        <a
          href="#"
          class="departure-stop-link"
          @click.stop.prevent="$emit('focusLeg', firstTransitLegIdx)"
          >{{ firstTransitLegDepartureLocation() }}</a
        >
      </template>
    </i18n-t>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { formatDuration } from 'src/utils/format';
import { i18n } from 'src/i18n/lang';
import Trip from 'src/models/Trip';
import { useNow } from 'src/utils/useNow';

/// "in 3 min & 12 min from Symphony": when a transit trip's departures leave, and from where.
export default defineComponent({
  name: 'TransitDeparture',
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    /// Every departure of the same trip, in order, when there's more than this one.
    departures: {
      type: Array as PropType<Trip[]>,
      default: undefined,
    },
  },
  emits: {
    focusLeg: (legIdx: number) => Number.isInteger(legIdx),
  },
  setup() {
    // Keeps "in 3 min" true, and moves on to later departures as earlier ones leave.
    return { nowTime: useNow() };
  },
  computed: {
    firstTransitLegIdx(): number {
      return this.trip.legs.findIndex((leg) => leg.transitLeg);
    },
  },
  methods: {
    /// "3 & 12 min", saying the unit once when both are in minutes.
    formattedTwoDepartures(next: number, after: number): string {
      const [nextMinutes, afterMinutes] = [next, after].map((seconds) =>
        Math.round(seconds / 60),
      );
      if (nextMinutes! >= 1 && afterMinutes! < 60) {
        return i18n.global.t('departures_$first_$second_minutes', {
          first: nextMinutes,
          second: afterMinutes,
        });
      }
      return i18n.global.t('departures_$first_$second', {
        first: formatDuration(next, 'shortform'),
        second: formatDuration(after, 'shortform'),
      });
    },
    firstTransitLegDepartureLocation(): string | undefined {
      return this.trip.firstTransitLeg?.departureLocationName;
    },
    /// "Departs in 3 & 12 min": the next two departures, or how long ago this one left once none
    /// are ahead.
    formattedDurationUntilStart(): string | undefined {
      const secondsUntil = (trip: Trip): number | undefined => {
        const startTime = trip.firstTransitLeg?.startTime;
        return startTime && (startTime.getTime() - this.nowTime) / 1000;
      };
      const departures = (this.departures ?? [this.trip]) as Trip[];
      const [next, after] = departures
        .map(secondsUntil)
        .filter((seconds) => seconds !== undefined && seconds >= 0)
        .slice(0, 2) as number[];
      if (next !== undefined) {
        const timeDuration =
          after === undefined
            ? formatDuration(next, 'shortform')
            : this.formattedTwoDepartures(next, after);
        return i18n.global.t('departs_$timeDuration_from_now', {
          timeDuration,
        });
      }
      const secondsSince = secondsUntil(this.trip as Trip);
      if (secondsSince === undefined) {
        return undefined;
      }
      return i18n.global.t('departs_$timeDuration_since_now', {
        timeDuration: formatDuration(-secondsSince, 'shortform'),
      });
    },
  },
});
</script>

<style lang="scss">
// One line, cut short, so a long stop name doesn't make the row taller: tapping it shows where.
.transit-departure {
  max-width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.real-time-departure-location {
  opacity: 0.8;
}
.departure-stop-link {
  color: inherit;
  text-decoration: underline;
  text-decoration-color: rgba(0, 0, 0, 0.3);
  text-underline-offset: 2px;
}
.real-time-departure-time {
  font-weight: 500;
}
</style>
