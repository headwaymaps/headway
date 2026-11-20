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
    <trip-corpus
      v-if="debugEnabled"
      url="/compare-routes/corpus.json"
      :from="fromPlace?.point"
      :to="toPlace?.point"
      @select="selectCorpusTrip"
    />
    <transit-unsupported-notice
      v-if="error?.errorCode == TripFetchErrorCode.UnsupportedTransitArea"
      :to-place="toPlace"
      :from-place="fromPlace"
      :transit-area-names="error.transitAreaNames"
      :uncovered-places="error.uncoveredPlaces"
    />
    <div v-else-if="error" class="search-error">
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
      <trip-group-list-item
        v-for="(group, idx) in tripGroups"
        :key="group.first.raw.startTime + (group.first.raw.tripKey ?? idx)"
        :group="group"
        :active="selectedTrip"
        @select="clickTrip"
        @steps="showTripSteps"
        @focus-leg="focusLeg"
      />
    </q-list>
  </div>
</template>
<script lang="ts">
import { getBaseMap, type BaseMapInterface } from 'src/components/BaseMap.vue';
import { defineComponent, markRaw, Ref, ref } from 'vue';
import Place, { PlaceId, PlaceStorage } from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import TripGroupListItem from 'src/components/TripGroupListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import TripCorpus from 'src/components/TripCorpus.vue';
import { debugEnabled } from 'src/utils/debug';
import TransitUnsupportedNotice from 'src/components/TransitUnsupportedNotice.vue';
import Trip, {
  fetchBestTrips,
  TripFetchError,
  TripFetchErrorCode,
} from 'src/models/Trip';
import TripGroup from 'src/models/TripGroup';
import TripLayerId from 'src/models/TripLayerId';
import VehicleOverlay from 'src/models/VehicleOverlay';
import Prefs from 'src/utils/Prefs';
import { GradeShades } from 'src/utils/grade';
import Markers, {
  focusTripStep,
  hideTripStepCallout,
  showSteepClimbBadges,
} from 'src/utils/Markers';
import { useRoute, type LocationQueryRaw } from 'vue-router';
import TransitQuery, { TransitQueryParams } from 'src/models/TransitQuery';
import { LngLat } from 'maplibre-gl';

export default defineComponent({
  name: 'AlternatesPage',
  components: {
    TripGroupListItem,
    TripSearch,
    TripCorpus,
    TransitUnsupportedNotice,
  },
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
    vehicleOverlay?: VehicleOverlay;
    TravelMode: typeof TravelMode;
    TripFetchErrorCode: typeof TripFetchErrorCode;
  } {
    return {
      trips: [],
      tripMarkers: [],
      error: undefined,
      activeTrip: undefined,
      isLoading: false,
      vehicleOverlay: undefined,
      TravelMode: TravelMode,
      TripFetchErrorCode: TripFetchErrorCode,
    };
  },
  computed: {
    debugEnabled: () => debugEnabled.value,
    // Reactive unwrapping loses the class's private members, so the template can't pass it as is.
    selectedTrip(): Trip | undefined {
      return this.activeTrip as Trip | undefined;
    },
    tripGroups(): TripGroup[] {
      return TripGroup.group(this.trips as Trip[]);
    },
  },
  watch: {
    mode: async function (): Promise<void> {
      await this.updateTrips();
    },
  },
  unmounted: function () {
    this.stopVehicleOverlay();
    showSteepClimbBadges([]);
    hideTripStepCallout();
  },
  mounted: async function () {
    [this.fromPlace, this.toPlace] = await Promise.all(
      [this.from, this.to].map((id) =>
        id == '_' ? undefined : PlaceStorage.fetchFromSerializedId(id),
      ),
    );
    await this.rewriteUrl();
  },
  methods: {
    errorText(error: TripFetchError): string {
      if (error.errorCode == TripFetchErrorCode.UnsupportedNonTransitArea) {
        return this.$t('routing_area_not_supported');
      } else if (error.message) {
        return this.$t('other_routing_error_with_$message', {
          message: error.message,
        });
      } else {
        return this.$t('routing_error_unknown');
      }
    },
    stopVehicleOverlay() {
      this.vehicleOverlay?.stop();
      this.vehicleOverlay = undefined;
    },
    /// Draw the vehicles serving the trips on screen, for the feeds that report where they are.
    startVehicleOverlay(
      map: BaseMapInterface,
      from: LngLat,
      to: LngLat,
      trips: Trip[],
      selected: Trip,
    ) {
      this.stopVehicleOverlay();
      const overlay = new VehicleOverlay(map, from, to, trips, (trip) =>
        this.clickTrip(trip),
      );
      this.vehicleOverlay = markRaw(overlay);
      overlay.start();
      overlay.selectTrip(selected);
    },
    /// Zooms to where a step of `trip` begins, and says what the step is - once the trip is
    /// selected. Until then, picking a step just picks its trip, like the rest of the row.
    focusLeg(trip: Trip, legIdx: number) {
      if (this.activeTrip !== trip) {
        this.clickTrip(trip);
        return;
      }
      const leg = trip.legs[legIdx]!;
      focusTripStep(leg.sourceLngLat, leg.stepDescription);
    },
    clickTrip(trip: Trip) {
      hideTripStepCallout();
      this.$data.activeTrip = trip;
      this.renderTrips(trip);
    },
    searchBoxDidSelectFromPlace(place?: Place) {
      this.fromPlace = place;
      this.rewriteUrl();
    },
    searchBoxDidSelectToPlace(place?: Place) {
      this.toPlace = place;
      this.rewriteUrl();
    },
    async selectCorpusTrip(from: LngLat, to: LngLat) {
      [this.fromPlace, this.toPlace] = await Promise.all(
        [from, to].map((p) => PlaceStorage.fetchFromId(PlaceId.location(p))),
      );
      await this.rewriteUrl();
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

      const query: LocationQueryRaw = this.transitQuery.searchQuery();
      // Keeps a debugging session's links shareable with the tools still on.
      if (debugEnabled.value) query.debug = null;
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
      this.stopVehicleOverlay();

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
          // The reactive copy, which is what the rows and their groups hold.
          this.renderTrips(this.trips[0] as Trip);
          this.startVehicleOverlay(
            map,
            this.fromPlace.point,
            this.toPlace.point,
            trips,
            trips[0]!,
          );
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
    /// Draws one line per group of departures, the selected trip's on top. Layers are keyed by
    /// group, so picking another departure of the same group redraws its line in place.
    renderTrips(selectedTrip: Trip) {
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly empty');
        return;
      }
      const groups = this.tripGroups as TripGroup[];
      const selectedIdx = groups.findIndex((group) =>
        group.includes(selectedTrip),
      );
      if (selectedIdx === -1) {
        console.error('selected trip is not among the trips');
        return;
      }
      this.activeTrip = selectedTrip;
      showSteepClimbBadges(selectedTrip.legs);

      for (let groupIdx = 0; groupIdx < groups.length; groupIdx++) {
        const trip = groups[groupIdx]!.first;
        for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
          const leg = trip.legs[legIdx]!;
          if (groupIdx == selectedIdx) {
            if (map.hasLayer(TripLayerId.unselectedLeg(groupIdx, legIdx))) {
              map.removeLayer(TripLayerId.unselectedLeg(groupIdx, legIdx));
            }
            continue;
          }

          for (const layerId of [
            TripLayerId.selectedLeg(groupIdx, legIdx),
            TripLayerId.legContext(groupIdx, legIdx),
            ...GradeShades.map((shade) =>
              TripLayerId.legSteep(groupIdx, legIdx, shade),
            ),
            TripLayerId.legRiddenStops(groupIdx, legIdx),
            TripLayerId.legContextStops(groupIdx, legIdx),
            TripLayerId.legOnOffStops(groupIdx, legIdx),
          ]) {
            if (map.hasLayer(layerId)) {
              map.removeLayer(layerId);
            }
          }

          if (map.hasLayer(TripLayerId.unselectedLeg(groupIdx, legIdx))) {
            continue;
          }

          const layerId = TripLayerId.unselectedLeg(groupIdx, legIdx);
          map.pushTripLayer(layerId, leg.geometry, leg.paintStyle(false));
          if (legIdx > 0 && !trip.transfersAtStop(legIdx)) {
            const transferLayerId = TripLayerId.legStart(groupIdx, legIdx);
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
          // Resolved when the line is clicked rather than when it was drawn: a later search
          // replaces the trips, and selecting one looks them up by identity.
          map.on('click', layerId.toString(), () => {
            // Cast since reactive unwrapping loses the class's private members.
            const clicked = (this.tripGroups as TripGroup[])[groupIdx];
            if (clicked) {
              this.clickTrip(clicked.nextDeparture(new Date()));
            }
          });
        }
      }

      // Added last to be sure it's on top of the unselected trips. A layer already drawn for
      // another departure of the group is updated rather than replaced.
      for (let legIdx = 0; legIdx < selectedTrip.legs.length; legIdx++) {
        const leg = selectedTrip.legs[legIdx]!;
        const pushOrRemove = (
          layerId: TripLayerId,
          layer: { geometry: GeoJSON.Geometry } | undefined,
          push: (layerId: TripLayerId) => void,
        ) => {
          if (layer) {
            push(layerId);
          } else if (map.hasLayer(layerId)) {
            map.removeLayer(layerId);
          }
        };
        // Pushed before the leg's own layer so the ridden portion draws over it.
        const context = leg.contextLayer();
        pushOrRemove(
          TripLayerId.legContext(selectedIdx, legIdx),
          context,
          (layerId) =>
            map.pushTripLayer(layerId, context!.geometry, context!.paint),
        );
        map.pushTripLayer(
          TripLayerId.selectedLeg(selectedIdx, legIdx),
          leg.selectedGeometry(),
          leg.paintStyle(true),
        );
        const steepSections = leg.steepSectionLayers();
        for (const shade of GradeShades) {
          const steep = steepSections.find((steep) => steep.shade === shade);
          pushOrRemove(
            TripLayerId.legSteep(selectedIdx, legIdx, shade),
            steep,
            (layerId) =>
              map.pushTripLayer(layerId, steep!.geometry, steep!.paint),
          );
        }
        // Pushed after both lines, so the dots sit on top of them.
        for (const [layerId, stops] of [
          [
            TripLayerId.legContextStops(selectedIdx, legIdx),
            leg.contextStopsLayer(),
          ],
          [
            TripLayerId.legRiddenStops(selectedIdx, legIdx),
            leg.riddenStopsLayer(),
          ],
          [
            TripLayerId.legOnOffStops(selectedIdx, legIdx),
            leg.onOffStopsLayer(),
          ],
        ] as const) {
          pushOrRemove(layerId, stops, (layerId) =>
            map.pushTripStopsLayer(layerId, stops!.geometry, stops!.paint),
          );
        }
      }
      getBaseMap()?.fitBounds(selectedTrip.bounds);

      this.vehicleOverlay?.selectTrip(selectedTrip);
    },
  },
});
</script>

<style lang="scss">
.search-error {
  padding: 16px;
}
</style>
