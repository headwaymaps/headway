<template>
  <trip-list-item
    :click-handler="() => $emit('select', shownTrip)"
    :active="active === shownTrip"
    :trip="shownTrip"
  >
    <template v-if="isTransit" #lead>
      <transit-departure
        :trip="shownTrip"
        :departures="group.trips"
        @focus-leg="(legIdx: number) => $emit('focusLeg', shownTrip, legIdx)"
      />
    </template>
    <multi-modal-list-item
      v-if="isTransit"
      :trip="shownTrip"
      :active="active === shownTrip"
      :leg-badges="group.legBadges"
      @focus-leg="(legIdx: number) => $emit('focusLeg', shownTrip, legIdx)"
    />
    <single-mode-list-item
      v-else
      :trip="shownTrip"
      :active="active === shownTrip"
    />
    <div v-if="group.trips.length > 1" class="departure-chips">
      <q-btn
        v-for="trip in group.trips"
        :key="trip.raw.startTime"
        :outline="trip !== shownTrip"
        :unelevated="trip === shownTrip"
        :color="trip === shownTrip ? 'primary' : 'black'"
        :label="trip.departureTimeFormatted"
        size="sm"
        dense
        no-caps
        padding="2px 8px"
        @click.stop="$emit('select', trip)"
      />
    </div>
    <template #action>
      <q-btn
        class="trip-list-item-action"
        :class="{ 'trip-list-item-action--unselected': active !== shownTrip }"
        unelevated
        no-caps
        padding="3px 10px"
        :label="$t('route_picker_steps_btn')"
        @click.stop="$emit('steps', shownTrip)"
      />
    </template>
  </trip-list-item>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import TripListItem from 'src/components/TripListItem.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';
import TransitDeparture from 'src/components/TransitDeparture.vue';
import Trip from 'src/models/Trip';
import TripGroup from 'src/models/TripGroup';
import { TravelMode } from 'src/utils/models';
import { useNow } from 'src/utils/useNow';

export default defineComponent({
  name: 'TripGroupListItem',
  components: {
    TripListItem,
    MultiModalListItem,
    SingleModeListItem,
    TransitDeparture,
  },
  props: {
    group: {
      type: Object as PropType<TripGroup>,
      required: true,
    },
    active: {
      type: Object as PropType<Trip>,
      default: undefined,
    },
  },
  emits: {
    select: (trip: Trip) => trip instanceof Trip,
    steps: (trip: Trip) => trip instanceof Trip,
    focusLeg: (trip: Trip, legIdx: number) =>
      trip instanceof Trip && Number.isInteger(legIdx),
  },
  setup() {
    return { now: useNow() };
  },
  computed: {
    isTransit(): boolean {
      return this.group.first.mode == TravelMode.Transit;
    },
    /// The row becomes whichever departure is picked, else the next one yet to leave.
    shownTrip(): Trip {
      const active = this.active as Trip | undefined;
      if (active && this.group.includes(active)) {
        return active;
      }
      return this.group.nextDeparture(new Date(this.now)) as Trip;
    },
  },
});
</script>

<style lang="scss">
.departure-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 6px 0 2px;
}
// Black text, but a lighter outline than black would draw.
.departure-chips .q-btn--outline::before {
  border-color: #9e9e9e;
}
</style>
