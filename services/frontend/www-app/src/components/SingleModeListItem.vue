<template>
  <div>
    <q-item-label v-if="trip.viaRoadsFormatted">
      {{ $t('via_$place', { place: trip.viaRoadsFormatted }) }}
    </q-item-label>
    <div v-if="trip.raw.routePreferences?.length" class="route-preferences">
      <q-badge
        v-for="preference in trip.raw.routePreferences"
        :key="preference"
        rounded
        :class="`route-preference hw-label route-preference--${preference}`"
      >
        <span class="route-preference-emoji" aria-hidden="true">
          {{ preference === 'quieter' ? '🦦' : '⚡' }}
        </span>
        {{ $t(`route_preference_${preference}`) }}
      </q-badge>
    </div>
    <elevation-chart
      v-if="elevation"
      v-model:scrub-fraction="scrubFraction"
      :profile="elevation.profile"
      :steep-sections="elevation.steepSections"
      :width="280"
      :height="60"
      :interactive="active"
      @select-steep-section="selectSteepClimb"
    />
    <elevation-totals
      v-if="elevation"
      class="chart-totals hw-caption"
      :climb-meters="elevation.totalClimbMeters"
      :fall-meters="elevation.totalFallMeters"
      :distance-units="trip.preferredDistanceUnits"
    />
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
.chart-totals {
  display: flex;
  // tucked up against the chart, past its padding and margin
  margin-top: -16px;
  margin-bottom: 4px;
}

.route-preferences {
  display: flex;
  gap: 4px;
  margin-top: 4px;
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
</style>
