<template>
  <div class="top-card">
    <trip-search
      :from-place="fromPlace"
      :to-place="toPlace"
      :current-mode="mode"
      :initial-search-time="searchTime"
      :initial-search-date="searchDate"
      :did-select-from-place="searchBoxDidSelectFromPlace"
      :did-select-to-place="searchBoxDidSelectToPlace"
      :did-swap-places="clickedSwap"
      :time-did-change="searchTimeDidChange"
      :date-did-change="searchDateDidChange"
    />
  </div>
  <div class="bottom-card">
    <div class="search-error" v-if="error">
      <p>
        {{ errorText(error) }}
      </p>
      <div v-if="error.transit">
        <router-link
          :to="{ name: 'alternates', params: { mode: 'car', to, from } }"
          >{{ $t('try_driving_directions') }}</router-link
        >
      </div>
    </div>
    <q-linear-progress v-if="isLoading" indeterminate />
    <q-list v-if="trips.length > 0">
      <trip-list-item
        v-for="trip in trips"
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
<style lang="scss">
.search-error {
  padding: 16px;
}
</style>

<script lang="ts">
import { getBaseMap } from 'src/components/BaseMap.vue';
import { Component, defineComponent, Ref, ref } from 'vue';
import Place, { PlaceStorage } from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import TripListItem from 'src/components/TripListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';
import Trip, { fetchBestTrips, TripFetchError } from 'src/models/Trip';
import TripLayerId from 'src/models/TripLayerId';
import Itinerary, { ItineraryErrorCode } from 'src/models/Itinerary';
import { RouteErrorCode } from 'src/models/Route';
import Prefs from 'src/utils/Prefs';
import Markers from 'src/utils/Markers';
import { useRoute } from 'vue-router';

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
  data(): {
    trips: Trip[];
    error?: TripFetchError;
    activeTrip?: Trip;
    isLoading: boolean;
    // only used by transit
    earliestStart: number;
    latestArrival: number;
  } {
    return {
      trips: [],
      error: undefined,
      activeTrip: undefined,
      isLoading: false,
      earliestStart: 0,
      latestArrival: 0,
    };
  },
  components: { TripListItem, TripSearch },
  methods: {
    errorText(error: TripFetchError): string {
      if (error.transit) {
        switch (error.itineraryError.errorCode) {
          case ItineraryErrorCode.SourceOutsideBounds:
            return this.$t('transit_area_not_supported_for_source');
          case ItineraryErrorCode.DestinationOutsideBounds:
            return this.$t('transit_area_not_supported_for_destination');
          case ItineraryErrorCode.TransitServiceDisabled:
            return this.$t('transit_routing_not_enabled');
          case ItineraryErrorCode.Other:
            return this.$t('transit_trip_error_unknown');
        }
      } else {
        switch (error.routeError.errorCode) {
          case RouteErrorCode.UnsupportedArea:
            return this.$t('routing_area_not_supported');
          case RouteErrorCode.Other:
            return this.$t('routing_error_unknown');
        }
      }
    },
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
    clickTrip(trip: Trip) {
      this.$data.activeTrip = trip;
      let index = this.$data.trips.indexOf(trip);
      if (index !== -1) {
        this.renderTrips(index);
      }
    },
    searchBoxDidSelectFromPlace(place?: Place) {
      this.fromPlace = place;
      this.rewriteUrl();
    },
    searchBoxDidSelectToPlace(place?: Place) {
      this.toPlace = place;
      this.rewriteUrl();
    },
    showTripSteps(trip: Trip) {
      let index = this.$data.trips.indexOf(trip);
      if (index !== -1 && this.to && this.from) {
        let path = `/directions/${this.mode}/${encodeURIComponent(
          this.to
        )}/${encodeURIComponent(this.from)}/${index}`;
        let query = this.dateTimeQuery();
        this.$router.push({ path, query });
      }
    },
    clickedSwap(newFromValue?: Place, newToValue?: Place) {
      this.fromPlace = newFromValue;
      this.toPlace = newToValue;
      this.rewriteUrl();
    },
    searchTimeDidChange(newValue: string) {
      this.searchTime = newValue;
      this.rewriteUrl();
    },
    searchDateDidChange(newValue: string) {
      this.searchDate = newValue;
      this.rewriteUrl();
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
    dateTimeQueryString(): string {
      let query = new URLSearchParams(this.dateTimeQuery());
      return query.toString();
    },
    rewriteUrl: async function () {
      if (!this.fromPlace && !this.toPlace) {
        this.$router.push('/');
        return;
      }

      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';

      let path = `/directions/${this.mode}/${toEncoded}/${fromEncoded}`;

      let queryString = this.dateTimeQueryString();
      if (queryString.length > 0) {
        path += '?' + queryString;
      }

      this.$router.push(path);
      await this.updateTrips();
    },
    async updateTrips(): Promise<void> {
      let map = getBaseMap();
      if (!map) {
        console.error('map was not set');
        return;
      }

      map.removeAllLayers();
      map.removeAllMarkers();

      if (this.fromPlace && this.toPlace) {
        this.isLoading = true;
        this.trips = [];
        this.error = undefined;
        this.activeTrip = undefined;

        const result = await fetchBestTrips(
          this.fromPlace.point,
          this.toPlace.point,
          this.mode,
          Prefs.stored.distanceUnits(this.fromPlace, this.toPlace),
          this.searchTime,
          this.searchDate
        ).finally(() => {
          this.isLoading = false;
        });

        if (result.ok) {
          const trips = result.value;
          this.calculateTransitStats(trips);
          this.trips = trips;
          this.renderTrips(0);
          this.error = undefined;
        } else {
          this.trips = [];
          this.error = result.error;
        }
      } else {
        this.trips = [];
        this.error = undefined;
        this.isLoading = false;
      }

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
    renderTrips(selectedIdx: number) {
      console.assert(this.trips.length > 0);
      const trips: Trip[] = this.trips;
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly empty');
        return;
      }
      this.$data.trips = trips;
      this.activeTrip = trips[selectedIdx];

      for (let tripIdx = 0; tripIdx < trips.length; tripIdx++) {
        const trip = trips[tripIdx];
        for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
          const leg = trip.legs[legIdx];
          if (tripIdx == selectedIdx) {
            if (map.hasLayer(TripLayerId.unselected(tripIdx, legIdx))) {
              map.removeLayer(TripLayerId.unselected(tripIdx, legIdx));
            }
            continue;
          }

          if (map.hasLayer(TripLayerId.selected(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.selected(tripIdx, legIdx));
          }

          if (map.hasLayer(TripLayerId.unselected(tripIdx, legIdx))) {
            continue;
          }

          let layerId = TripLayerId.unselected(tripIdx, legIdx);
          map.pushTripLayer(layerId, leg.geometry(), leg.paintStyle(false));
          map.on('mouseover', layerId.toString(), () => {
            map.setCursor('pointer');
          });
          map.on('mouseout', layerId.toString(), () => {
            map.setCursor('');
          });
          map.on('click', layerId.toString(), () => {
            this.clickTrip(trip);
          });
        }
      }

      // Add selected trip last to be sure it's on top of the unselected trips
      const selectedTrip = trips[selectedIdx];
      for (let legIdx = 0; legIdx < selectedTrip.legs.length; legIdx++) {
        const leg = selectedTrip.legs[legIdx];
        if (!map.hasLayer(TripLayerId.selected(selectedIdx, legIdx))) {
          map.pushTripLayer(
            TripLayerId.selected(selectedIdx, legIdx),
            leg.geometry(),
            leg.paintStyle(true)
          );
        }
      }
      getBaseMap()?.fitBounds(selectedTrip.bounds);
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
    },
  },
  mounted: async function () {
    if (this.to != '_') {
      this.toPlace = await PlaceStorage.fetchFromSerializedId(
        this.to as string
      );
    }
    if (this.from != '_') {
      this.fromPlace = await PlaceStorage.fetchFromSerializedId(
        this.from as string
      );
    }
    await this.rewriteUrl();
  },
  setup: function () {
    const toPlace: Ref<Place | undefined> = ref(undefined);
    const fromPlace: Ref<Place | undefined> = ref(undefined);
    const searchTime: Ref<string | undefined> = ref(undefined);
    const searchDate: Ref<string | undefined> = ref(undefined);

    const route = useRoute();
    if (typeof route.query.searchTime == 'string') {
      searchTime.value = route.query.searchTime;
    }
    if (typeof route.query.searchDate == 'string') {
      searchDate.value = route.query.searchDate;
    }

    return {
      toPlace,
      fromPlace,
      searchTime,
      searchDate,
    };
  },
});
</script>
