<template>
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
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { valhallaTypeToIcon } from 'src/services/ValhallaAPI';
import { getBaseMap } from './BaseMap.vue';
import Trip from 'src/models/Trip';
import { NonTransitLeg, TravelmuxManeuver } from 'src/services/TravelmuxClient';
import Markers from 'src/utils/Markers';

export default defineComponent({
  name: 'SingleModeSteps',
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
  } {
    // this cast is safe because we know that the trip is a non-transit trip
    const nonTransitLeg = this.trip.legs[0]?.raw.nonTransitLeg as NonTransitLeg;
    console.assert(nonTransitLeg);
    return {
      selectedManeuver: undefined,
      nonTransitLeg,
      geometry: this.trip.legs[0]!.geometry,
    };
  },
  methods: {
    valhallaTypeToIcon,
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
