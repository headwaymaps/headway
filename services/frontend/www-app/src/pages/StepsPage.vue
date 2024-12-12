<template>
  <div class="top-card">
    <div
      style="display: flex; flex-direction: row; gap: 16px; align-items: center"
    >
      <q-btn round icon="arrow_back" @click="() => goToAlternates()" />
      <div style="display: flex; flex-direction: column; gap: 8px; flex: 1">
        <search-box
          :hint="$t('search.from')"
          :style="{ flex: 1 }"
          :initial-place="fromPlace"
          readonly
        />
        <search-box
          :hint="$t('search.to')"
          :style="{ flex: 1 }"
          :initial-place="toPlace"
          readonly
        />
      </div>
    </div>
  </div>
  <div class="bottom-card steps-page-bottom-card">
    <component :is="componentForMode(trip.mode)" v-if="trip" :trip="trip" />
  </div>
</template>

<script lang="ts">
import { getBaseMap } from 'src/components/BaseMap.vue';
import { TravelMode, DistanceUnits } from 'src/utils/models';
import Place, { PlaceStorage } from 'src/models/Place';
import { defineComponent, Component, Ref, ref } from 'vue';
import Trip, { fetchBestTrips } from 'src/models/Trip';
import SingleModeSteps from 'src/components/SingleModeSteps.vue';
import MultiModalSteps from 'src/components/MultiModalSteps.vue';
import SearchBox from 'src/components/SearchBox.vue';
import TripLayerId from 'src/models/TripLayerId';
import Markers from 'src/utils/Markers';
import { useRoute } from 'vue-router';
import TransitQuery from 'src/models/TransitQuery';

export default defineComponent({
  name: 'StepsPage',
  components: { SearchBox },
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
    tripIdx: {
      type: String,
      required: true,
    },
  },
  setup: function () {
    const toPlace: Ref<Place | undefined> = ref(undefined);
    const fromPlace: Ref<Place | undefined> = ref(undefined);

    const route = useRoute();
    const transitQuery: Ref<TransitQuery> = ref(
      TransitQuery.parseFromQuery(route.query),
    );

    return {
      toPlace,
      fromPlace,
      transitQuery,
    };
  },
  data: function (): {
    trip?: Trip;
    tripMarkers: string[];
  } {
    return {
      trip: undefined,
      tripMarkers: [],
    };
  },
  mounted: async function () {
    this.toPlace = await PlaceStorage.fetchFromSerializedId(
      this.$props.to as string,
    );
    this.fromPlace = await PlaceStorage.fetchFromSerializedId(
      this.$props.from as string,
    );
    await this.rewriteUrl();

    const map = getBaseMap();
    if (!map) {
      console.error('map was not set');
      return;
    }

    setTimeout(() => {
      const map = getBaseMap();
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

    map.removeMarkersExcept(this.tripMarkers);
    if (this.fromPlace) {
      map.pushMarker(
        'source_marker',
        Markers.tripStart().setLngLat(this.fromPlace.point),
      );
    }

    if (this.toPlace) {
      map.pushMarker(
        'destination_marker',
        Markers.tripEnd().setLngLat(this.toPlace.point),
      );
    }
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
      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';
      const path = `/directions/${this.mode}/${toEncoded}/${fromEncoded}`;
      const query = this.transitQuery.searchQuery();
      this.$router.push({ path, query });
    },

    rewriteUrl: async function () {
      const map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }

      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';
      this.$router.push(
        `/directions/${this.mode}/${toEncoded}/${fromEncoded}/${this.tripIdx}`,
      );

      if (this.fromPlace && this.toPlace) {
        const result = await fetchBestTrips(
          this.fromPlace.point,
          this.toPlace.point,
          this.mode,
          this.fromPlace.preferredDistanceUnits() ?? DistanceUnits.Kilometers,
          this.transitQuery.params.searchTime,
          this.transitQuery.params.searchDate,
          this.transitQuery.params.arriveBy,
          this.transitQuery.params.transitWithBicycle,
        );
        if (!result.ok) {
          console.error('fetchBestTrips.error', result.error);
          this.goToAlternates();
          return;
        }

        const trips = result.value;
        const idx = parseInt(this.tripIdx);
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
      const tripIdx = parseInt(this.tripIdx);

      // TODO: add a map.filterLayers((layerName: string) => boolean) method so
      // we can keep the layer we need and remove the others based on a prefix/regex/w.e.
      const layerIds = [];
      for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
        const leg = trip.legs[legIdx];

        const layerId = TripLayerId.selectedLeg(tripIdx, legIdx);
        layerIds.push(layerId);

        if (!map.hasLayer(layerId)) {
          map.pushTripLayer(layerId, leg.geometry, leg.paintStyle(true));
        }

        const transferLayerId = TripLayerId.legStart(tripIdx, legIdx);
        if (
          legIdx > 0 &&
          !this.tripMarkers.includes(transferLayerId.toString())
        ) {
          this.tripMarkers.push(transferLayerId.toString());
          if (!map.hasMarker(transferLayerId.toString())) {
            map.pushMarker(
              transferLayerId.toString(),
              Markers.transfer().setLngLat(leg.start),
            );
          }
        }
      }
      map.removeLayersExcept(layerIds);
    },
  },
});
</script>

<style lang="scss">
.steps-page-bottom-card {
  @media screen and (max-width: 799px) {
    max-height: calc(100% - 350px);
  }
}
</style>
