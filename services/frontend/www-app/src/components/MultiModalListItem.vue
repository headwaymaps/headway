<template>
  <q-item-label class="hw-heading">
    {{ trip.startStopTimesFormatted }}
  </q-item-label>
  <q-item-label class="trip-legs">
    <span
      v-for="(legIdx, shownIdx) in shownLegIndices"
      :key="legIdx"
      class="trip-leg"
    >
      <q-icon v-if="shownIdx > 0" name="chevron_right" class="trip-leg-arrow" />
      <route-pill
        :badge="badges[legIdx]!"
        :real-time="trip.legs[legIdx]!.realTime"
        :has-alerts="trip.legs[legIdx]!.alerts.length > 0"
        @click.stop="$emit('focusLeg', legIdx)"
      />
    </span>
  </q-item-label>
  <div v-if="formattedDurationUntilStart() !== undefined">
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
  <transit-alert-list
    v-if="trip.hasAlerts"
    :groups="trip.alertGroups"
    :hidden="!active"
  />
</template>
<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { formatDuration } from 'src/utils/format';
import { i18n } from 'src/i18n/lang';
import Trip from 'src/models/Trip';
import TripGroup, { LegBadge } from 'src/models/TripGroup';
import RoutePill from 'src/components/RoutePill.vue';
import { useNow } from 'src/utils/useNow';
import TransitAlertList from 'src/components/TransitAlertList.vue';

export default defineComponent({
  name: 'MultiModalListItem',
  components: { RoutePill, TransitAlertList },
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    active: {
      type: Boolean,
      required: true,
    },
    /// Badges for the legs that cover every departure in the trip's group.
    legBadges: {
      type: Array as PropType<LegBadge[]>,
      default: undefined,
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
    badges(): LegBadge[] {
      return (
        this.legBadges ?? TripGroup.group([this.trip as Trip])[0]!.legBadges
      );
    },
    /// The legs worth a pill: a short walk is too small to be worth the room, and one at either
    /// end of the trip more so.
    shownLegIndices(): number[] {
      const lastIdx = this.badges.length - 1;
      return this.badges.flatMap((badge, idx) => {
        if (badge.kind != 'selfPowered') {
          return [idx];
        }
        const atEnd = idx == 0 || idx == lastIdx;
        const tooShort = badge.minutes <= (atEnd ? 2 : 1);
        return tooShort ? [] : [idx];
      });
    },
  },
  methods: {
    firstTransitLegDepartureLocation(): string | undefined {
      return this.trip.firstTransitLeg?.departureLocationName;
    },
    /// "in 3 min & 12 min": the next two departures, or how long ago this one left once none are
    /// ahead.
    formattedDurationUntilStart(): string | undefined {
      const secondsUntil = (trip: Trip): number | undefined => {
        const startTime = trip.firstTransitLeg?.startTime;
        return startTime && (startTime.getTime() - this.nowTime) / 1000;
      };
      const departures = (this.departures ?? [this.trip]) as Trip[];
      const upcoming = departures
        .map(secondsUntil)
        .filter((seconds) => seconds !== undefined && seconds >= 0)
        .slice(0, 2)
        .map((seconds) => formatDuration(seconds!, 'shortform'));
      const [next, after] = upcoming;
      if (next) {
        const timeDuration = after
          ? i18n.global.t('departures_$first_$second', {
              first: next,
              second: after,
            })
          : next;
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
.trip-legs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  // Room for the badges hanging above a wrapped row's pills.
  row-gap: 8px;
  padding-top: 4px;
}
.trip-leg {
  display: inline-flex;
  align-items: center;
  gap: 2px;
}
.trip-leg-arrow {
  color: $hw-dark-gray;
  // Clear of the badges hanging off the previous pill's corner.
  margin: 0 1px 0 5px;
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
