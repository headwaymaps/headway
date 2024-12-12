<template>
  <q-list>
    <q-item
      v-for="maneuver in nonTransitLeg.maneuvers"
      :key="JSON.stringify(maneuver)"
      class="maneuver"
      active-class="bg-blue-1"
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

export default defineComponent({
  name: 'SingleModeSteps',
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
  },
  data(): {
    geometry: GeoJSON.LineString;
    nonTransitLeg: NonTransitLeg;
  } {
    // this cast is safe because we know that the trip is a non-transit trip
    const nonTransitLeg = this.trip.legs[0]?.raw.nonTransitLeg as NonTransitLeg;
    console.assert(nonTransitLeg);
    return {
      nonTransitLeg,
      geometry: this.trip.legs[0].geometry,
    };
  },
  methods: {
    valhallaTypeToIcon,
    clickedManeuver: function (maneuver: TravelmuxManeuver) {
      getBaseMap()?.flyTo(maneuver.startPoint, { zoom: 16 });
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
