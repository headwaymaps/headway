<template>
  <div>
    <elevation-chart
      v-if="(isBicycle || isWalking) && elevationData.length > 0"
      :elevations="elevationData"
      :total-climb-meters="totalClimbMeters"
      :total-fall-meters="totalFallMeters"
      :distance-units="trip.preferredDistanceUnits"
      :width="300"
      :height="100"
    />
    <q-list>
      <q-item
        v-for="maneuver in nonTransitLeg.maneuvers"
        :key="JSON.stringify(maneuver)"
        class="maneuver"
        active-class="list-item--selected"
        :active="selectedManeuver === maneuver"
        clickable
        @click="clickedManeuver(maneuver)"
      >
        <q-item-section avatar>
          <q-icon :name="valhallaTypeToIcon(maneuver.type)" />
        </q-item-section>
        <q-item-section>
          <q-item-label>
            {{ maneuver.instruction }}
          </q-item-label>
          <q-item-label caption>
            {{ maneuver.verbalPostTransitionInstruction }}
          </q-item-label>
        </q-item-section>
      </q-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { valhallaTypeToIcon } from 'src/services/ValhallaAPI';
import { getBaseMap } from './BaseMap.vue';
import Trip from 'src/models/Trip';
import {
  NonTransitLeg,
  TravelmuxManeuver,
  TravelmuxClient,
  TravelmuxMode,
} from 'src/services/TravelmuxClient';
import Markers from 'src/utils/Markers';
import ElevationChart from './ElevationChart.vue';

export default defineComponent({
  name: 'SingleModeSteps',
  components: {
    ElevationChart,
  },
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
  },
  data(): {
    selectedManeuver: TravelmuxManeuver | undefined;
    geometry: GeoJSON.LineString;
    nonTransitLeg: NonTransitLeg;
    elevationData: number[];
    totalClimbMeters: number;
    totalFallMeters: number;
  } {
    // this cast is safe because we know that the trip is a non-transit trip
    const nonTransitLeg = this.trip.legs[0]?.raw.nonTransitLeg as NonTransitLeg;
    console.assert(nonTransitLeg);
    return {
      selectedManeuver: undefined,
      nonTransitLeg,
      geometry: this.trip.legs[0]!.geometry,
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
    valhallaTypeToIcon,
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
    clickedManeuver: function (maneuver: TravelmuxManeuver) {
      this.selectedManeuver = maneuver;
      const baseMap = getBaseMap();
      if (baseMap) {
        baseMap.flyTo(maneuver.startPoint, { zoom: 16 });

        // Add a marker for the maneuver location
        const icon = valhallaTypeToIcon(maneuver.type);
        const marker = Markers.maneuver(icon, maneuver.bearingBefore || 0);
        marker.setRotationAlignment('map');
        marker.setLngLat(maneuver.startPoint);
        baseMap.pushMarker('selected-maneuver', marker);
      }
    },
  },
});
</script>

<style lang="scss">
.maneuver {
  padding-top: 10px;
  padding-bottom: 10px;
  border-bottom: solid 1px #ddd;
}
</style>
