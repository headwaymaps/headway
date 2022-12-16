<template>
  <div class="top-card">
    <div
      style="display: flex; flex-direction: row; gap: 16px; align-items: center"
    >
      <q-btn round icon="arrow_back" v-on:click="() => goToAlternates()" />
      <div style="display: flex; flex-direction: column; gap: 8px; flex: 1">
        <search-box
          :hint="$t('search.from')"
          :style="{ flex: 1 }"
          :force-text="fromPlace?.name"
          readonly
        />
        <search-box
          :hint="$t('search.to')"
          :style="{ flex: 1 }"
          :force-text="toPlace?.name"
          readonly
        />
      </div>
    </div>
  </div>
  <div class="bottom-card steps-page-bottom-card">
    <component v-if="trip" :is="componentForMode(trip.mode)" :trip="trip" />
  </div>
</template>

<style lang="scss">
.steps-page-bottom-card {
  @media screen and (max-width: 800px) {
    max-height: calc(100% - 350px);
  }
}
</style>

<script lang="ts">
import {
  destinationMarker,
  getBaseMap,
  sourceMarker,
} from 'src/components/BaseMap.vue';
import { TravelMode, DistanceUnits } from 'src/utils/models';
import Place, { PlaceStorage } from 'src/models/Place';
import { defineComponent, Component, Ref, ref } from 'vue';
import Trip, { fetchBestTrips } from 'src/models/Trip';
import SingleModeSteps from 'src/components/SingleModeSteps.vue';
import MultiModalSteps from 'src/components/MultiModalSteps.vue';
import SearchBox from 'src/components/SearchBox.vue';

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
  components: { SearchBox },
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
