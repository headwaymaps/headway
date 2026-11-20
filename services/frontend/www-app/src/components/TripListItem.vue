<template>
  <q-item
    class="list-item trip-list-item"
    :clickable="!$props.active"
    active-class="list-item--selected"
    :active="$props.active"
    @click="$props.clickHandler"
  >
    <q-item-section top>
      <div class="trip-list-item-header">
        <q-item-label class="hw-figure trip-list-item-duration">{{
          trip.durationFormatted
        }}</q-item-label>
        <q-badge
          v-for="preference in trip.raw.routePreferences ?? []"
          :key="preference"
          rounded
          :class="`route-preference hw-label route-preference--${preference}`"
        >
          <span class="route-preference-emoji" aria-hidden="true">
            {{ preference === 'quieter' ? '🦦' : '⚡' }}
          </span>
          {{ $t(`route_preference_${preference}`) }}
        </q-badge>
        <q-space />
        <slot name="action" />
      </div>
      <slot name="lead" />
      <q-item-label class="hw-secondary">{{ summaryFormatted }}</q-item-label>
      <slot />
    </q-item-section>
  </q-item>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { QItemProps } from 'quasar';
import Trip from 'src/models/Trip';
import { TravelMode } from 'src/utils/models';
import { formatTime } from 'src/utils/format';

export default defineComponent({
  name: 'TripListItem',
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    active: Boolean,
    clickHandler: {
      type: Function as PropType<QItemProps['onClick']>,
      default: () => {},
    },
  },
  computed: {
    /// "5:13 PM ETA · 3.1 mi". A transit trip's distance says little about it, so it goes without.
    summaryFormatted(): string {
      const eta = this.$t('trip_eta_$time', {
        time: formatTime(this.trip.endTime),
      });
      return this.trip.mode == TravelMode.Transit
        ? eta
        : `${eta} · ${this.trip.distanceFormatted}`;
    },
  },
});
</script>

<style lang="scss">
.trip-list-item-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.trip-list-item-duration {
  font-size: 1.125rem;
  line-height: 1.2;
}
.trip-list-item-action {
  // Level with the top of the duration.
  align-self: flex-start;
  background: $hw-go;
  color: white;
  font-size: 14px;
  font-weight: 600;
  border-radius: 6px;
}
// Quieter on the rows not picked, so the durations read first.
.trip-list-item-action--unselected {
  background: transparent;
  color: $hw-go;
  box-shadow: inset 0 0 0 1px $hw-go;
}
.route-preference {
  padding: 5px 10px;
}
.route-preference-emoji {
  font-size: 1.3em;
  line-height: 0;
  margin-right: 4px;
  vertical-align: -0.1em;
}
.route-preference--quieter {
  background: $hw-teal-100;
  color: $hw-teal-700;
}
.route-preference--faster {
  background: $hw-amber-100;
  color: $hw-amber-700;
}
// Outlined rather than shaded on hover: a gray fill would swallow the gray walking pills.
@media (any-hover: hover) {
  .trip-list-item.q-hoverable:hover {
    box-shadow: inset 0 0 0 2px rgba(0, 0, 0, 0.2);
  }
  .trip-list-item.q-hoverable:hover > .q-focus-helper {
    opacity: 0;
  }
}
</style>
