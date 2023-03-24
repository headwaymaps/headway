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
    tripIdx: {
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
      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';
      let path = `/directions/${this.mode}/${toEncoded}/${fromEncoded}`;
      let query = this.dateTimeQuery();
      this.$router.push({ path, query });
    },

    rewriteUrl: async function () {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }

      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';
      this.$router.push(
        `/directions/${this.mode}/${toEncoded}/${fromEncoded}/${this.tripIdx}`
      );

      if (this.fromPlace && this.toPlace) {
        const result = await fetchBestTrips(
          this.fromPlace.point,
          this.toPlace.point,
          this.mode,
          this.fromPlace.preferredDistanceUnits() ?? DistanceUnits.Kilometers,
          this.searchTime,
          this.searchDate
        );
        if (!result.ok) {
          console.error('fetchBestTrips.error', result.error);
          this.goToAlternates();
          return;
        }

        let trips = result.value;
        let idx = parseInt(this.tripIdx);
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
      let layerIds = [];
      for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
        const leg = trip.legs[legIdx];

        const layerId = TripLayerId.selected(tripIdx, legIdx);
        layerIds.push(layerId);

        if (!map.hasLayer(layerId)) {
          map.pushTripLayer(layerId, leg.geometry(), leg.paintStyle(true));
        }
      }
      map.removeLayersExcept(layerIds);
    },
    dateTimeQuery(): Record<string, string> {
      let query: Record<string, string> = {};
      if (this.searchDate) {
        query['searchDate'] = this.searchDate;
      }
      if (this.searchTime) {
        query['searchTime'] = this.searchTime;
      }
      return query;
    },
  },
  mounted: async function () {
    this.toPlace = await PlaceStorage.fetchFromSerializedId(
      this.$props.to as string
    );
    this.fromPlace = await PlaceStorage.fetchFromSerializedId(
      this.$props.from as string
    );
    if (typeof this.$route.query.searchTime == 'string') {
      this.searchTime = this.$route.query.searchTime;
    }
    if (typeof this.$route.query.searchDate == 'string') {
      this.searchDate = this.$route.query.searchDate;
    }

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
    if (this.fromPlace) {
      map.pushMarker(
        'source_marker',
        Markers.tripStart().setLngLat(this.fromPlace.point)
      );
    }

    if (this.toPlace) {
      map.pushMarker(
        'destination_marker',
        Markers.tripEnd().setLngLat(this.toPlace.point)
      );
    }
  },
  setup: function () {
    const toPlace: Ref<Place | undefined> = ref(undefined);
    const fromPlace: Ref<Place | undefined> = ref(undefined);
    const searchTime: Ref<string | undefined> = ref(undefined);
    const searchDate: Ref<string | undefined> = ref(undefined);

    return {
      toPlace,
      fromPlace,
      searchTime,
      searchDate,
    };
  },
});
</script>
