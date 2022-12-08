<template>
  <q-item-label>
    {{ item.startStopTimesFormatted() }}
  </q-item-label>
  <q-item-label caption>
    {{ item.viaRouteFormatted }}
  </q-item-label>
  <q-item-label caption :hidden="!active">
    {{
      $t('walk_distance', {
        preformattedDistance: item.walkingDistanceFormatted(),
      })
    }}
  </q-item-label>
  <q-item-label :hidden="active && areStepsVisible">
    <q-btn
      style="margin-left: -6px"
      padding="6px"
      flat
      icon="directions"
      label="Details"
      size="sm"
      v-on:click="showSteps"
    />
  </q-item-label>
  <transit-steps :hidden="!(active && areStepsVisible)" :itinerary="item" />
</template>
<script lang="ts">
import Itinerary from 'src/models/Itinerary';
import { defineComponent, PropType } from 'vue';
import TransitSteps from './TransitSteps.vue';

export default defineComponent({
  name: 'MultiModalListItem',
  props: {
    item: {
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
    // MultiModalListItem actually doesn't use this, but SingleModeListItem needs it, so
    // we have to include it here to avoid an "unexpected property" warning.
    // This feels gross, but hopefully I can find a better way.
    showRouteSteps: Function,
  },
  data(): {
    areStepsVisible: boolean;
  } {
    return {
      areStepsVisible: false,
    };
  },
  methods: {
    showSteps() {
      this.areStepsVisible = true;
      console.log(this.item);
    },
  },
  components: { TransitSteps },
});
</script>
