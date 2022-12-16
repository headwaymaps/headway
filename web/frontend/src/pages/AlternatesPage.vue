<template>
  <trip-search
    :from-poi="fromPoi"
    :to-poi="toPoi"
    :current-mode="mode"
    :did-select-from-poi="searchBoxDidSelectFromPoi"
    :did-select-to-poi="searchBoxDidSelectToPoi"
    :did-swap-pois="clickedSwap"
  />
  <div class="bottom-card bg-white" ref="bottomCard" v-if="fromPoi && toPoi">
    <q-list>
      <trip-list-item
        v-for="trip in $data.trips"
        :click-handler="() => clickTrip(trip)"
        :active="$data.activeTrip === trip"
        :duration-formatted="trip.durationFormatted"
        :distance-formatted="trip.lengthFormatted"
        v-bind:key="JSON.stringify(trip)"
      >
        <component
          :is="componentForMode(trip.mode)"
          :trip="trip"
          :active="trip === activeTrip"
          :earliest-start="earliestStart"
          :latest-arrival="latestArrival"
        />
        <q-item-label>
          <q-btn
            style="margin-left: -6px"
            padding="6px"
            flat
            icon="directions"
            :label="$t('route_picker_show_route_details_btn')"
            size="sm"
            v-on:click="showTripSteps(trip)"
          />
        </q-item-label>
      </trip-list-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import {
  destinationMarker,
  sourceMarker,
  getBaseMap,
  setBottomCardAllowance,
} from 'src/components/BaseMap.vue';
import {
  canonicalizePoi,
  decanonicalizePoi,
  DistanceUnits,
  POI,
} from 'src/utils/models';
import { poiDisplayName } from 'src/i18n/utils';
import { Component, defineComponent, Ref, ref } from 'vue';
import Place from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import TripListItem from 'src/components/TripListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';
import Trip, { fetchBestTrips } from 'src/models/Trip';
import { toLngLat } from 'src/utils/geomath';
import Itinerary from 'src/models/Itinerary';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'AlternatesPage',
  props: {
    mode: {
      type: String as () => TravelMode,
      required: true,
    },
    to: String,
    from: String,
  },
  data: function (): {
    trips: Trip[];
    activeTrip: Trip | undefined;
    // only used by transit
    earliestStart: number;
    latestArrival: number;
  } {
    return {
      trips: [],
      activeTrip: undefined,
      earliestStart: 0,
      latestArrival: 0,
    };
  },
  components: { TripListItem, TripSearch },
  methods: {
    componentForMode(mode: TravelMode): Component {
      switch (mode) {
        case TravelMode.Walk:
        case TravelMode.Bike:
        case TravelMode.Drive:
          return SingleModeListItem;
        case TravelMode.Transit:
          return MultiModalListItem;
      }
    },
    poiDisplayName,
    clickTrip(trip: Trip) {
      this.$data.activeTrip = trip;
      let index = this.$data.trips.indexOf(trip);
      if (index !== -1) {
        this.renderTrips(this.$data.trips, index);
      }
    },
    searchBoxDidSelectFromPoi(poi?: POI) {
      this.fromPoi = poi;
      this.rewriteUrl();
    },
    searchBoxDidSelectToPoi(poi?: POI) {
      this.toPoi = poi;
      this.rewriteUrl();
    },
    showTripSteps(trip: Trip) {
      let index = this.$data.trips.indexOf(trip);
      if (index !== -1 && this.to && this.from) {
        this.$router.push(
          `/directions/${this.mode}/${encodeURIComponent(
            this.to
          )}/${encodeURIComponent(this.from)}/${index}`
        );
      }
    },
    clickedSwap(newFromValue?: POI, newToValue?: POI) {
      fromPoi.value = newFromValue;
      toPoi.value = newToValue;
      this.rewriteUrl();
    },
    rewriteUrl: async function () {
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }
      const fromCanonical = fromPoi.value
        ? canonicalizePoi(fromPoi.value)
        : '_';
      const toCanonical = toPoi.value ? canonicalizePoi(toPoi.value) : '_';
      this.$router.push(
        '/directions/' +
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          encodeURIComponent(this.mode!) +
          '/' +
          encodeURIComponent(toCanonical) +
          '/' +
          encodeURIComponent(fromCanonical)
      );
      await this.updateTrips();
    },

    async updateTrips(): Promise<void> {
      getBaseMap()?.removeAllLayers();
      getBaseMap()?.removeAllMarkers();
      if (fromPoi.value?.position && toPoi.value?.position) {
        const fromCanonical = canonicalizePoi(fromPoi.value);
        // TODO: replace POI with Place so we don't have to hit pelias twice?
        let fromPlace = await Place.fetchFromSerializedId(fromCanonical);
        const trips = await fetchBestTrips(
          toLngLat(fromPoi.value.position),
          toLngLat(toPoi.value.position),
          this.mode,
          fromPlace.preferredDistanceUnits() ?? DistanceUnits.Kilometers
        );
        this.calculateTransitStats(trips);
        this.renderTrips(trips, 0);
      }
    },
    renderTrips(trips: Trip[], selectedIdx: number) {
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly empty');
        return;
      }
      this.$data.trips = trips;
      this.activeTrip = trips[selectedIdx];

      if (fromPoi.value?.position) {
        map.pushMarker(
          'source_marker',
          sourceMarker().setLngLat([
            fromPoi.value.position.long,
            fromPoi.value.position.lat,
          ])
        );
      }
      if (toPoi.value?.position) {
        map.pushMarker(
          'destination_marker',
          destinationMarker().setLngLat([
            toPoi.value.position.long,
            toPoi.value.position.lat,
          ])
        );
      }

      const unselectedLayerName = (tripIdx: number, legIdx: number) =>
        `alternate_${this.mode}_${tripIdx}.${legIdx}_unselected`;
      const selectedLayerName = (tripIdx: number, legIdx: number) =>
        `alternate_${this.mode}_${tripIdx}.${legIdx}_selected`;

      for (let tripIdx = 0; tripIdx < trips.length; tripIdx++) {
        const trip = trips[tripIdx];
        for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
          const leg = trip.legs[legIdx];
          if (tripIdx == selectedIdx) {
            if (map.hasLayer(unselectedLayerName(tripIdx, legIdx))) {
              map.removeLayer(unselectedLayerName(tripIdx, legIdx));
            }
            continue;
          }

          if (map.hasLayer(selectedLayerName(tripIdx, legIdx))) {
            map.removeLayer(selectedLayerName(tripIdx, legIdx));
          }

          if (map.hasLayer(unselectedLayerName(tripIdx, legIdx))) {
            continue;
          }

          map.pushTripLayer(
            unselectedLayerName(tripIdx, legIdx),
            leg.geometry(),
            leg.paintStyle(false)
          );
        }
      }

      // Add selected trip last to be sure it's on top of the unselected trips
      const selectedTrip = trips[selectedIdx];
      for (let legIdx = 0; legIdx < selectedTrip.legs.length; legIdx++) {
        const leg = selectedTrip.legs[legIdx];
        if (!map.hasLayer(selectedLayerName(selectedIdx, legIdx))) {
          map.pushTripLayer(
            selectedLayerName(selectedIdx, legIdx),
            leg.geometry(),
            leg.paintStyle(true)
          );
        }
      }
      setTimeout(async () => {
        this.resizeMap();
      });
      getBaseMap()?.fitBounds(selectedTrip.bounds);
    },
    resizeMap() {
      if (this.$refs.bottomCard && this.$refs.bottomCard) {
        setBottomCardAllowance(
          (this.$refs.bottomCard as HTMLDivElement).offsetHeight
        );
      } else {
        setBottomCardAllowance(0);
      }
    },
    calculateTransitStats(trips: Trip[]) {
      this.$data.earliestStart = Number.MAX_SAFE_INTEGER;
      this.$data.latestArrival = 0;
      // terrible hack.
      if (this.mode != TravelMode.Transit) {
        return;
      }

      let itineraries: Itinerary[] = trips as Itinerary[];

      for (var index = 0; index < itineraries.length; index++) {
        this.$data.earliestStart = Math.min(
          this.$data.earliestStart,
          itineraries[index].startTime
        );
        this.$data.latestArrival = Math.max(
          this.$data.latestArrival,
          itineraries[index].endTime
        );
      }
    },
  },
  watch: {
    mode: async function (): Promise<void> {
      await this.updateTrips();
      this.resizeMap();
    },
  },
  unmounted: function () {
    getBaseMap()?.removeLayersExcept([]);
  },
  mounted: async function () {
    setTimeout(async () => {
      toPoi.value = await decanonicalizePoi(this.$props.to as string);
      fromPoi.value = await decanonicalizePoi(this.$props.from as string);
      await this.rewriteUrl();
      this.resizeMap();
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
