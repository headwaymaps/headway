<template>
  <q-item-label>
    {{ trip.startStopTimesFormatted() }}
  </q-item-label>
  <q-item-label caption>
    {{ trip.viaRouteFormatted }}
  </q-item-label>
  <q-item-label caption :hidden="!active">
    {{
      $t('walk_distance', {
        preformattedDistance: trip.walkingDistanceFormatted(),
      })
    }}
  </q-item-label>
  <transit-steps :hidden="!(active && areStepsVisible)" :itinerary="trip" />
</template>
<script lang="ts">
import Itinerary from 'src/models/Itinerary';
import { defineComponent, PropType } from 'vue';
import TransitSteps from './TransitSteps.vue';

export default defineComponent({
  name: 'MultiModalListItem',
  props: {
    trip: {
      type: Object as PropType<Itinerary>,
      required: true,
    },
    active: {
      type: Boolean,
      required: true,
    },
    earliestStart: {
      type: Number,
      required: true,
    },
    latestArrival: {
      type: Number,
      required: true,
    },
  },
  data(): {
    areStepsVisible: boolean;
  } {
    return {
      areStepsVisible: false,
    };
  },
  components: { TransitSteps },
});
</script>
