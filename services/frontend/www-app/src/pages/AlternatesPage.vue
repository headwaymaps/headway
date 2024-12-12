<template>
  <div class="top-card">
    <trip-search
      :from-place="fromPlace"
      :to-place="toPlace"
      :current-mode="mode"
      :initial-search="transitQuery.params"
      :did-select-from-place="searchBoxDidSelectFromPlace"
      :did-select-to-place="searchBoxDidSelectToPlace"
      :did-swap-places="clickedSwap"
      :search-did-change="searchDidChange"
    />
  </div>
  <div class="bottom-card">
    <div v-if="error" class="search-error">
      <p>
        {{ errorText(error) }}
      </p>
      <div v-if="error && mode == TravelMode.Transit">
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
        :key="JSON.stringify(trip)"
        :click-handler="
          /* why is this cast necessary? */ () => clickTrip(trip as Trip)
        "
        :active="$data.activeTrip === trip"
        :duration-formatted="trip.durationFormatted"
        :distance-formatted="trip.distanceFormatted"
      >
        <component
          :is="componentForMode(trip.mode)"
          :trip="trip"
          :active="trip === activeTrip"
        />
        <q-item-label>
          <q-btn
            style="margin-left: -6px"
            padding="6px"
            flat
            icon="directions"
            :label="$t('route_picker_show_route_details_btn')"
            size="sm"
            @click="
              /* why is this cast necessary? */ showTripSteps(trip as Trip)
            "
          />
        </q-item-label>
      </trip-list-item>
    </q-list>
  </div>
</template>
<script lang="ts">
import { getBaseMap } from 'src/components/BaseMap.vue';
import { Component, defineComponent, Ref, ref } from 'vue';
import Place, { PlaceStorage } from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import TripListItem from 'src/components/TripListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';
import Trip, {
  fetchBestTrips,
  TripFetchError,
  TripFetchErrorCode,
} from 'src/models/Trip';
import TripLayerId from 'src/models/TripLayerId';
import Prefs from 'src/utils/Prefs';
import Markers from 'src/utils/Markers';
import { useRoute } from 'vue-router';
import TransitQuery, { TransitQueryParams } from 'src/models/TransitQuery';

export default defineComponent({
  name: 'AlternatesPage',
  components: { TripListItem, TripSearch },
  props: {
    mode: {
      type: String as () => TravelMode,
      required: true,
    },
    to: {
      type: String,
      default: '_',
    },
    from: {
      type: String,
      default: '_',
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
  data(): {
    trips: Trip[];
    tripMarkers: string[];
    error?: TripFetchError;
    activeTrip?: Trip;
    isLoading: boolean;
    TravelMode: typeof TravelMode;
  } {
    return {
      trips: [],
      tripMarkers: [],
      error: undefined,
      activeTrip: undefined,
      isLoading: false,
      TravelMode: TravelMode,
    };
  },
  watch: {
    mode: async function (): Promise<void> {
      await this.updateTrips();
    },
  },
  mounted: async function () {
    if (this.to != '_') {
      this.toPlace = await PlaceStorage.fetchFromSerializedId(
        this.to as string,
      );
    }
    if (this.from != '_') {
      this.fromPlace = await PlaceStorage.fetchFromSerializedId(
        this.from as string,
      );
    }
    await this.rewriteUrl();
  },
  methods: {
    errorText(error: TripFetchError): string {
      switch (error.errorCode) {
        case TripFetchErrorCode.UnsupportedNonTransitArea:
          return this.$t('routing_area_not_supported');
        case TripFetchErrorCode.UnsupportedTransitArea:
          return this.$t('transit_routing_area_not_supported');
        case TripFetchErrorCode.Other:
          if (error.message) {
            return this.$t('other_routing_error_with_$message', {
              message: error.message,
            });
          } else {
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
        default:
          throw new Error(`unexpected mode: ${mode ?? 'none'}`);
      }
    },
    clickTrip(trip: Trip) {
      this.$data.activeTrip = trip;
      const index = this.$data.trips.indexOf(trip);
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
      const index = this.$data.trips.indexOf(trip);
      if (index !== -1 && this.to && this.from) {
        const path = `/directions/${this.mode}/${encodeURIComponent(
          this.to,
        )}/${encodeURIComponent(this.from)}/${index}`;
        const query = this.transitQuery.searchQuery();
        this.$router.push({ path, query });
      }
    },
    clickedSwap(newFromValue?: Place, newToValue?: Place) {
      this.fromPlace = newFromValue;
      this.toPlace = newToValue;
      this.rewriteUrl();
    },
    searchDidChange(newValue: TransitQueryParams) {
      this.transitQuery = new TransitQuery(newValue);
      this.rewriteUrl();
    },

    rewriteUrl: async function () {
      if (!this.fromPlace && !this.toPlace) {
        this.$router.push('/');
        return;
      }

      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';

      const path = `/directions/${this.mode}/${toEncoded}/${fromEncoded}`;

      const query = this.transitQuery.searchQuery();
      this.$router.push({ path, query });
      await this.updateTrips();
    },
    async updateTrips(): Promise<void> {
      const map = getBaseMap();
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
          this.transitQuery.params.searchTime,
          this.transitQuery.params.searchDate,
          this.transitQuery.params.arriveBy,
          this.transitQuery.params.transitWithBicycle,
        ).finally(() => {
          this.isLoading = false;
        });

        if (result.ok) {
          const trips = result.value;
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
    renderTrips(selectedIdx: number) {
      console.assert(this.trips.length > 0);
      const trips: Trip[] = this.trips as Trip[];
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
            if (map.hasLayer(TripLayerId.unselectedLeg(tripIdx, legIdx))) {
              map.removeLayer(TripLayerId.unselectedLeg(tripIdx, legIdx));
            }
            continue;
          }

          if (map.hasLayer(TripLayerId.selectedLeg(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.selectedLeg(tripIdx, legIdx));
          }

          if (map.hasLayer(TripLayerId.unselectedLeg(tripIdx, legIdx))) {
            continue;
          }

          const layerId = TripLayerId.unselectedLeg(tripIdx, legIdx);
          map.pushTripLayer(layerId, leg.geometry, leg.paintStyle(false));
          if (legIdx > 0) {
            const transferLayerId = TripLayerId.legStart(tripIdx, legIdx);
            map.pushMarker(
              transferLayerId.toString(),
              Markers.transfer().setLngLat(leg.start),
            );
          }
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
        if (!map.hasLayer(TripLayerId.selectedLeg(selectedIdx, legIdx))) {
          map.pushTripLayer(
            TripLayerId.selectedLeg(selectedIdx, legIdx),
            leg.geometry,
            leg.paintStyle(true),
          );
        }
      }
      getBaseMap()?.fitBounds(selectedTrip.bounds);
    },
  },
});
</script>

<style lang="scss">
.search-error {
  padding: 16px;
}
</style>
