<template>
  <div class="top-left-card"></div>
  <q-btn
    round
    icon="arrow_back"
    class="top-left-fab"
    v-on:click="() => goToAlternates()"
  />
  <div class="bottom-card steps-page-bottom-card bg-white" ref="bottomCard">
    <component v-if="trip" :is="componentForMode(trip.mode)" :trip="trip" />
  </div>
</template>

<style lang="scss">
.steps-page-bottom-card {
  max-height: calc(100% - 200px);
}
</style>

<script lang="ts">
import {
  destinationMarker,
  getBaseMap,
  setBottomCardAllowance,
  sourceMarker,
} from 'src/components/BaseMap.vue';
import { TravelMode, DistanceUnits } from 'src/utils/models';
import Place, { PlaceStorage } from 'src/models/Place';
import { defineComponent, Component, Ref, ref } from 'vue';
import Trip, { fetchBestTrips } from 'src/models/Trip';
import SingleModeSteps from 'src/components/SingleModeSteps.vue';
import MultiModalSteps from 'src/components/MultiModalSteps.vue';

let toPlace: Ref<Place | undefined> = ref(undefined);
let fromPlace: Ref<Place | undefined> = ref(undefined);

export default defineComponent({
  name: 'StepsPage',
  props: {
    mode: {
      type: String as () => TravelMode,
      required: true,
    },
    to: {
      type: String,
      required: true,
    },
    from: {
      type: String,
      required: true,
    },
    alternateIndex: {
      type: String,
      required: true,
    },
  },
  data: function (): {
    trip?: Trip;
  } {
    return {
      trip: undefined,
    };
  },
  methods: {
    componentForMode(mode: TravelMode): Component {
      switch (mode) {
        case TravelMode.Walk:
        case TravelMode.Bike:
        case TravelMode.Drive:
          return SingleModeSteps;
        case TravelMode.Transit:
          return MultiModalSteps;
      }
    },

    goToAlternates() {
      const fromEncoded = fromPlace.value?.urlEncodedId() ?? '_';
      const toEncoded = toPlace.value?.urlEncodedId() ?? '_';
      this.$router.push(`/directions/${this.mode}/${toEncoded}/${fromEncoded}`);
    },

    rewriteUrl: async function () {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }

      const fromEncoded = fromPlace.value?.urlEncodedId() ?? '_';
      const toEncoded = toPlace.value?.urlEncodedId() ?? '_';
      this.$router.push(
        `/directions/${this.mode}/${toEncoded}/${fromEncoded}/${this.alternateIndex}`
      );

      if (fromPlace.value && toPlace.value) {
        const result = await fetchBestTrips(
          fromPlace.value.point,
          toPlace.value.point,
          this.mode,
          fromPlace.value.preferredDistanceUnits() ?? DistanceUnits.Kilometers
        );
        if (!result.ok) {
          console.error('fetchBestTrips.error', result.error);
          this.goToAlternates();
          return;
        }

        let trips = result.value;
        let idx = parseInt(this.alternateIndex);
        const trip = trips[idx];
        console.assert(trip);
        this.$data.trip = trip;
        this.renderTripLayer();
      }
    },
    renderTripLayer() {
      const map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }
      const trip = this.trip;
      if (!trip) {
        console.error('trip was not set');
        return;
      }

      // TODO: add a map.filterLayers((layerName: string) => boolean) method so
      // we can keep the layer we need and remove the others based on a prefix/regex/w.e.
      map.removeAllLayers();

      for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
        const leg = trip.legs[legIdx];
        map.pushTripLayer(
          `selected_trip_leg_${legIdx}`,
          leg.geometry(),
          leg.paintStyle(true)
        );
      }
    },
    resizeMap() {
      if (!this.$refs.bottomCard) {
        console.error('bottom card was missing');
        setBottomCardAllowance(0);
        return;
      }

      setBottomCardAllowance(
        (this.$refs.bottomCard as HTMLDivElement).offsetHeight
      );
    },
  },
  mounted: async function () {
    toPlace.value = await PlaceStorage.fetchFromSerializedId(
      this.$props.to as string
    );
    fromPlace.value = await PlaceStorage.fetchFromSerializedId(
      this.$props.from as string
    );

    await this.rewriteUrl();

    let map = getBaseMap();
    if (!map) {
      console.error('map was not set');
      return;
    }
    this.resizeMap();

    setTimeout(() => {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }
      if (!this.trip) {
        console.error('trip was not set');
        return;
      }
      map.fitBounds(this.trip.bounds);
    });

    map.removeAllMarkers();
    if (fromPlace.value) {
      map.pushMarker(
        'source_marker',
        sourceMarker().setLngLat(fromPlace.value.point)
      );
    }

    if (toPlace.value) {
      map.pushMarker(
        'destination_marker',
        destinationMarker().setLngLat(toPlace.value.point)
      );
    }
  },
  setup: function () {
    return {
      toPlace,
      fromPlace,
    };
  },
});
</script>
