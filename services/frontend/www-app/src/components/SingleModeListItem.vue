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
        :class="`route-preference route-preference--${preference}`"
      >
        <span class="route-preference-emoji" aria-hidden="true">
          {{ preference === 'quieter' ? '🦦' : '⚡' }}
        </span>
        {{ $t(`route_preference_${preference}`) }}
      </q-badge>
    </div>
    <elevation-chart
      v-if="(isBicycle || isWalking) && elevationData.length > 0"
      :elevations="elevationData"
      :total-climb-meters="totalClimbMeters"
      :total-fall-meters="totalFallMeters"
      :distance-units="trip.preferredDistanceUnits"
      :width="280"
      :height="60"
    />
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import Trip from 'src/models/Trip';
import { TravelmuxClient, TravelmuxMode } from 'src/services/TravelmuxClient';
import ElevationChart from './ElevationChart.vue';

export default defineComponent({
  name: 'SingleModeListItem',
  components: {
    ElevationChart,
  },
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    // SingleModalListItem actually doesn't use this, but MultiModalListItem needs it, so
    // we have to include it here to avoid an "unexpected property" warning.
    // This feels gross, but hopefully I can find a better way.
    active: Boolean,
  },
  data(): {
    elevationData: number[];
    totalClimbMeters: number;
    totalFallMeters: number;
  } {
    return {
      elevationData: [],
      totalClimbMeters: 0,
      totalFallMeters: 0,
    };
  },
  computed: {
    isBicycle(): boolean {
      return this.trip.legs[0]?.raw.mode === TravelmuxMode.Bike;
    },
    isWalking(): boolean {
      return this.trip.legs[0]?.raw.mode === TravelmuxMode.Walk;
    },
  },
  mounted() {
    if (this.isBicycle || this.isWalking) {
      this.fetchElevationData();
    }
  },
  methods: {
    async fetchElevationData() {
      if (!this.trip.legs[0]) return;

      const pathGeometry = this.trip.legs[0].raw.geometry;
      const result = await TravelmuxClient.fetchElevation(pathGeometry);

      if (result.ok) {
        this.elevationData = result.value.elevation;
        this.totalClimbMeters = result.value.totalClimbMeters;
        this.totalFallMeters = result.value.totalFallMeters;
      } else {
        console.error('Failed to fetch elevation data:', result.error);
      }
    },
  },
});
</script>

<style scoped lang="scss">
.route-preferences {
  display: flex;
  gap: 4px;
  margin-top: 4px;
}

.route-preference {
  color: white;
  padding: 5px 10px;
  font-weight: 500;
}

.route-preference-emoji {
  font-size: 1.3em;
  line-height: 0;
  margin-right: 4px;
  vertical-align: -0.1em;
}

.route-preference--quieter {
  background: #5a9b5e;
}

.route-preference--faster {
  background: #d66a62;
}
</style>
