<template>
  <div class="top-left-card"></div>
  <q-btn
    round
    icon="arrow_back"
    class="top-left-fab"
    v-on:click="() => onBackClicked()"
  />
  <div
    class="bottom-card steps-page-bottom-card bg-white"
    ref="bottomCard"
    v-if="fromPoi && toPoi"
  >
    <component v-if="trip" :is="componentForMode(trip.mode)" :trip="trip" />
  </div>
</template>

<style lang="scss">
.steps-page-bottom-card {
  max-height: calc(100% - 200px);
}
</style>

<script lang="ts">
import { getBaseMap, setBottomCardAllowance } from 'src/components/BaseMap.vue';
import {
  encodePoi,
  canonicalizePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
  TravelMode,
  DistanceUnits,
} from 'src/utils/models';
import Place from 'src/models/Place';
import { defineComponent, Component, Ref, ref } from 'vue';
import { Marker } from 'maplibre-gl';
import { toLngLat } from 'src/utils/geomath';
import Trip, { fetchBestTrips } from 'src/models/Trip';
import SingleModeSteps from 'src/components/SingleModeSteps.vue';
import MultiModalSteps from 'src/components/MultiModalSteps.vue';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

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
    poiDisplayName,
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

    onBackClicked() {
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }
      const fromCanonical = fromPoi.value ? encodePoi(fromPoi.value) : '_';
      const toCanonical = toPoi.value ? encodePoi(toPoi.value) : '_';
      this.$router.push(
        `/directions/${this.mode}/${toCanonical}/${fromCanonical}`
      );
    },
    rewriteUrl: async function () {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }

      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }
      const fromCanonical = fromPoi.value
        ? canonicalizePoi(fromPoi.value)
        : '_';
      const toEncoded = toPoi.value ? encodePoi(toPoi.value) : '_';
      this.$router.push(
        `/directions/${this.mode}/${toEncoded}/${encodeURIComponent(
          fromCanonical
        )}/${this.alternateIndex}`
      );
      if (fromPoi.value?.position && toPoi.value?.position) {
        // TODO: replace POI with Place so we don't have to hit pelias twice?
        let fromPlace = await Place.fetchFromSerializedId(fromCanonical);
        const trips = await fetchBestTrips(
          toLngLat(fromPoi.value.position),
          toLngLat(toPoi.value.position),
          this.mode,
          fromPlace.preferredDistanceUnits() ?? DistanceUnits.Kilometers
        );
        let idx = parseInt(this.alternateIndex);
        let trip = trips[idx];
        console.assert(trip);
        this.$data.trip = trip;
        this.renderTripLayer(trip);
      }
    },
    renderTripLayer(trip: Trip) {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
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
      map.fitBounds(trip.bounds);
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
    setTimeout(async () => {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }

      toPoi.value = await decanonicalizePoi(this.$props.to as string);
      fromPoi.value = await decanonicalizePoi(this.$props.from as string);
      await this.rewriteUrl();
      this.resizeMap();

      getBaseMap()?.removeAllMarkers();
      if (this.toPoi?.position) {
        const marker = new Marker({ color: '#111111' }).setLngLat([
          this.toPoi.position.long,
          this.toPoi.position.lat,
        ]);
        getBaseMap()?.pushMarker('active_marker', marker);
      }
    });
  },
  setup: function () {
    return {
      toPoi,
      fromPoi,
    };
  },
});
</script>
