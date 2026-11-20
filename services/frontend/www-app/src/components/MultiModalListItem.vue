<template>
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
  <transit-alert-list
    v-if="trip.hasAlerts"
    :groups="trip.alertGroups"
    :hidden="!active"
  />
</template>
<script lang="ts">
import { defineComponent, PropType } from 'vue';
import Trip from 'src/models/Trip';
import TripGroup, { LegBadge } from 'src/models/TripGroup';
import RoutePill from 'src/components/RoutePill.vue';
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
  },
  emits: {
    focusLeg: (legIdx: number) => Number.isInteger(legIdx),
  },
  computed: {
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
</style>
