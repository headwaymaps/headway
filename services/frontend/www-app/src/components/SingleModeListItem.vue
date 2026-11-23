<template>
  <div>
    <div v-if="elevation" class="chart-row">
      <elevation-chart
        v-model:scrub-fraction="scrubFraction"
        :profile="elevation.profile"
        :steep-sections="elevation.steepSections"
        :width="280"
        :height="60"
        :interactive="active"
        @select-steep-section="selectSteepClimb"
      />
      <elevation-totals
        class="chart-totals hw-caption"
        :climb-meters="elevation.totalClimbMeters"
        :fall-meters="elevation.totalFallMeters"
        :distance-units="trip.preferredDistanceUnits"
      />
    </div>
    <q-item-label v-if="trip.viaRoadsFormatted" class="hw-secondary">
      {{ trip.viaRoadsFormatted }}
    </q-item-label>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import Trip from 'src/models/Trip';
import ElevationChart from './ElevationChart.vue';
import ElevationTotals from './ElevationTotals.vue';
import { useLegElevation } from 'src/composables/useLegElevation';

export default defineComponent({
  name: 'SingleModeListItem',
  components: {
    ElevationChart,
    ElevationTotals,
  },
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    active: Boolean,
  },
  setup(props) {
    return useLegElevation(props.trip.legs[0]!);
  },
  watch: {
    active(isActive: boolean) {
      if (!isActive) {
        this.scrubFraction = null;
      }
    },
  },
});
</script>

<style scoped lang="scss">
.chart-row {
  display: flex;
  align-items: center;
  gap: 4px;
  // Into the chart's own margins, which leave room the list row doesn't need.
  margin: -6px 0 -8px;
}

// Stacked beside the chart rather than under it, which would take another line.
.chart-totals {
  flex-direction: column;
  gap: 2px;
}
</style>
