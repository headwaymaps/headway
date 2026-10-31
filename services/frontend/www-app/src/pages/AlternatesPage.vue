<template>
  <div class="top-card">
    <trip-search
      :from-place="fromPlace"
      :to-place="toPlace"
      :current-mode="mode"
      :initial-search="transitQuery.params"
      :vias="vias"
      :did-remove-via="(index: number) => removeVia(index)"
      :did-select-from-place="searchBoxDidSelectFromPlace"
      :did-select-to-place="searchBoxDidSelectToPlace"
      :did-swap-places="clickedSwap"
      :search-did-change="searchDidChange"
    />
  </div>
  <div class="bottom-card">
    <trip-corpus
      v-if="debugEnabled"
      :from="fromPlace?.point"
      :to="toPlace?.point"
      @select="selectCorpusTrip"
    />
    <route-scoring
      v-if="debugEnabled && routeScoringEnabled"
      :trip="activeTrip as Trip"
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
import {
  baseMapPromise,
  getBaseMap,
  map as rawMap,
  type BaseMapInterface,
} from 'src/components/BaseMap.vue';
import { Component, defineComponent, markRaw, Ref, ref } from 'vue';
import Place, { PlaceId, PlaceStorage } from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import TripListItem from 'src/components/TripListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import TripCorpus from 'src/components/TripCorpus.vue';
import RouteScoring from 'src/components/RouteScoring.vue';
import { debugEnabled, routeScoringEnabled } from 'src/utils/debug';
import TransitUnsupportedNotice from 'src/components/TransitUnsupportedNotice.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';
import Trip, {
  fetchBestTrips,
  TripFetchError,
  TripFetchErrorCode,
} from 'src/models/Trip';
import TripLayerId from 'src/models/TripLayerId';
import VehicleOverlay from 'src/models/VehicleOverlay';
import Prefs from 'src/utils/Prefs';
import { GradeShades } from 'src/utils/grade';
import Markers, { showSteepClimbBadges } from 'src/utils/Markers';
import { useRoute, type LocationQueryRaw } from 'vue-router';
import TransitQuery, { TransitQueryParams } from 'src/models/TransitQuery';
import { LngLat, Marker, Popup } from 'maplibre-gl';
import type { MapMouseEvent, Point } from 'maplibre-gl';
import { debounce } from 'lodash';

/// How near, in pixels, a press has to land to a route to drag it.
const ROUTE_GRAB_PIXELS = 5;

export default defineComponent({
  name: 'AlternatesPage',
  components: {
    TripListItem,
    TripSearch,
    TripCorpus,
    RouteScoring,
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
      viaMarkers: [] as Marker[],
      editSeq: 0,
      /// Says what dragging or clicking what's under the mouse will do.
      editTooltip: new Popup({
        closeButton: false,
        closeOnClick: false,
        className: 'route-edit-tooltip',
        offset: 14,
      }),
      /// Whether a route or via is being dragged, which the tooltip would only get in the way of.
      dragging: false,
      /// Replans as a via is dragged, once it's paused for a moment.
      replanSoon: debounce((replan: () => void) => replan(), 250),
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
    vias(): LngLat[] {
      return this.transitQuery.params.vias ?? [];
    },
    debugEnabled: () => debugEnabled.value,
    routeScoringEnabled: () => routeScoringEnabled.value,
  },
  watch: {
    mode: async function (): Promise<void> {
      await this.updateTrips();
    },
    debugEnabled: async function (): Promise<void> {
      await this.updateTrips();
    },
  },
  unmounted: function () {
    this.stopVehicleOverlay();
    showSteepClimbBadges([]);
    this.replanSoon.cancel();
    rawMap?.off('mousedown', this.startViaDrag);
    rawMap?.off('mousemove', this.hoverRoute);
    rawMap?.getCanvas().removeEventListener('mouseleave', this.hideTooltip);
    this.hideTooltip();
    for (const marker of this.viaMarkers) marker.remove();
  },
  mounted: async function () {
    await baseMapPromise;
    rawMap!.on('mousedown', this.startViaDrag);
    rawMap!.on('mousemove', this.hoverRoute);
    rawMap!.getCanvas().addEventListener('mouseleave', this.hideTooltip);
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
    clickTrip(trip: Trip) {
      this.$data.activeTrip = trip;
      const index = this.$data.trips.indexOf(trip);
      if (index !== -1) {
        this.renderTrips(index);
      }
    },
    searchBoxDidSelectFromPlace(place?: Place) {
      this.fromPlace = place;
      this.forgetVias();
      this.rewriteUrl();
    },
    searchBoxDidSelectToPlace(place?: Place) {
      this.toPlace = place;
      this.forgetVias();
      this.rewriteUrl();
    },
    async selectCorpusTrip(from: LngLat, to: LngLat) {
      [this.fromPlace, this.toPlace] = await Promise.all(
        [from, to].map((p) => PlaceStorage.fetchFromId(PlaceId.location(p))),
      );
      this.forgetVias();
      await this.rewriteUrl();
    },
    /// Vias belong to the trip they were dragged on.
    forgetVias() {
      this.transitQuery = new TransitQuery({
        ...this.transitQuery.params,
        vias: undefined,
      });
    },
    /// The route lines on the map, which can be dragged to add a via.
    routeLayerIds(): string[] {
      return this.trips.flatMap((trip, tripIdx) =>
        trip.legs.flatMap((_, legIdx) =>
          [
            TripLayerId.selectedLeg(tripIdx, legIdx),
            TripLayerId.unselectedLeg(tripIdx, legIdx),
          ]
            .map((id) => id.toString())
            .filter((id) => rawMap?.getLayer(id)),
        ),
      );
    },
    /// Whether a route can be grabbed at `point`.
    isNearRoute({ x, y }: Point): boolean {
      const r = ROUTE_GRAB_PIXELS;
      return (
        rawMap!.queryRenderedFeatures(
          [
            [x - r, y - r],
            [x + r, y + r],
          ],
          { layers: this.routeLayerIds() },
        ).length > 0
      );
    },
    showTooltip(at: LngLat, text: string) {
      this.editTooltip.setLngLat(at).setText(text).addTo(rawMap!);
    },
    hideTooltip() {
      this.editTooltip.remove();
    },
    hoverRoute(e: MapMouseEvent) {
      if (this.dragging) return;
      // A via's marker has its own tooltip.
      if (isOnMarker(e)) return;
      if (this.isNearRoute(e.point)) {
        this.showTooltip(e.lngLat, this.$t('route_edit_drag_route'));
      } else {
        this.hideTooltip();
      }
    },
    /// Dragging any route adds a via where it's let go, between the stops it detours least from.
    startViaDrag(e: MapMouseEvent) {
      const map = rawMap!;
      const from = this.fromPlace?.point;
      const to = this.toPlace?.point;
      if (!from || !to) return;
      // A via's own marker drags that via.
      if (isOnMarker(e)) return;
      if (!this.isNearRoute(e.point)) return;
      e.preventDefault();
      this.dragging = true;
      this.hideTooltip();
      const r = ROUTE_GRAB_PIXELS;

      const marker = Markers.via(false).setLngLat(e.lngLat).addTo(map);
      marker.getElement().style.pointerEvents = 'none';
      const before = this.vias;
      const vias = ({ lngLat }: MapMouseEvent) =>
        withVia(from, to, before, lngLat);
      const follow = (move: MapMouseEvent) => {
        marker.setLngLat(move.lngLat);
        this.previewVias(vias(move));
      };
      map.on('mousemove', follow);
      map.once('mouseup', (up: MapMouseEvent) => {
        this.dragging = false;
        map.off('mousemove', follow);
        marker.remove();
        if (up.point.dist(e.point) >= r) this.setVias(vias(up));
        else this.replanSoon.cancel();
      });
    },
    removeVia(index: number) {
      this.setVias(this.vias.filter((_, i) => i !== index));
    },
    previewVias(vias: LngLat[]) {
      this.replanSoon(() => this.replan(vias));
    },
    setVias(vias: LngLat[]) {
      this.replanSoon.cancel();
      this.transitQuery = new TransitQuery({
        ...this.transitQuery.params,
        vias,
      });
      this.$router.replace({ query: this.urlQuery() });
      this.drawViaMarkers();
      void this.replan(vias);
    },
    drawViaMarkers() {
      for (const marker of this.viaMarkers) marker.remove();
      this.viaMarkers = this.vias.map((via, i) => {
        const marker = Markers.via(true).setLngLat(via).addTo(rawMap!);
        const moved = () =>
          this.vias.map((v, j) => (j === i ? marker.getLngLat() : v));
        // A drag ends in a click too, which shouldn't also remove the via.
        let dragged = false;
        marker.on('dragstart', () => {
          dragged = true;
          this.dragging = true;
          this.hideTooltip();
        });
        marker.on('drag', () => this.previewVias(moved()));
        marker.on('dragend', () => {
          this.dragging = false;
          this.setVias(moved());
        });
        const element = marker.getElement();
        element.addEventListener('mousedown', () => (dragged = false));
        element.addEventListener('mouseenter', () => {
          if (!this.dragging) {
            this.showTooltip(
              marker.getLngLat(),
              this.$t('route_edit_remove_via'),
            );
          }
        });
        element.addEventListener('mouseleave', this.hideTooltip);
        element.addEventListener('click', (e) => {
          e.stopPropagation();
          if (dragged) {
            dragged = false;
            return;
          }
          this.hideTooltip();
          this.removeVia(i);
        });
        return marker;
      });
    },
    /// Replaces the trips with ones through `vias`, keeping the map's view as it is.
    async replan(vias: LngLat[]) {
      const map = getBaseMap();
      if (!map || !this.fromPlace || !this.toPlace) return;
      const from = this.fromPlace.point;
      const to = this.toPlace.point;
      const seq = ++this.editSeq;
      const result = await fetchBestTrips(
        from,
        to,
        this.mode,
        Prefs.stored.distanceUnits(this.fromPlace, this.toPlace),
        { ...this.transitQuery.params, vias },
        debugEnabled.value,
      );
      if (seq !== this.editSeq) return;
      if (!result.ok) {
        this.error = result.error;
        return;
      }
      this.error = undefined;
      const trips = result.value;
      if (trips.length === 0) return;
      map.removeAllLayers();
      map.removeMarkersExcept(['source_marker', 'destination_marker']);
      this.trips = trips;
      this.renderTrips(0, false);
      this.startVehicleOverlay(map, from, to, trips, trips[0]!);
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

      this.$router.push({ path, query: this.urlQuery() });
      await this.updateTrips();
    },
    urlQuery(): LocationQueryRaw {
      const query: LocationQueryRaw = this.transitQuery.searchQuery();
      // Keeps a debugging session's links shareable with the tools still on.
      if (debugEnabled.value) query.debug = null;
      return query;
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
      this.drawViaMarkers();

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
          this.transitQuery.params,
          debugEnabled.value,
        ).finally(() => {
          this.isLoading = false;
        });

        if (result.ok) {
          const trips = result.value;
          this.trips = trips;
          this.renderTrips(0);
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
    renderTrips(selectedIdx: number, refit = true) {
      console.assert(this.trips.length > 0);
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly empty');
        return;
      }
      const trips: Trip[] = this.trips as Trip[];
      const selectedTrip = trips[selectedIdx]!;
      if (!selectedTrip) {
        console.error('selected trip was unexpectedly undefined');
        return;
      }
      this.$data.trips = trips;
      this.activeTrip = selectedTrip;
      showSteepClimbBadges(selectedTrip.legs);

      for (let tripIdx = 0; tripIdx < trips.length; tripIdx++) {
        const trip = trips[tripIdx]!;
        for (let legIdx = 0; legIdx < trip.legs.length; legIdx++) {
          const leg = trip.legs[legIdx]!;
          if (tripIdx == selectedIdx) {
            if (map.hasLayer(TripLayerId.unselectedLeg(tripIdx, legIdx))) {
              map.removeLayer(TripLayerId.unselectedLeg(tripIdx, legIdx));
            }
            continue;
          }

          if (map.hasLayer(TripLayerId.selectedLeg(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.selectedLeg(tripIdx, legIdx));
          }

          if (map.hasLayer(TripLayerId.legContext(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.legContext(tripIdx, legIdx));
          }

          for (const shade of GradeShades) {
            const steepLayerId = TripLayerId.legSteep(tripIdx, legIdx, shade);
            if (map.hasLayer(steepLayerId)) {
              map.removeLayer(steepLayerId);
            }
          }

          if (map.hasLayer(TripLayerId.legRiddenStops(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.legRiddenStops(tripIdx, legIdx));
          }

          if (map.hasLayer(TripLayerId.legContextStops(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.legContextStops(tripIdx, legIdx));
          }

          if (map.hasLayer(TripLayerId.legOnOffStops(tripIdx, legIdx))) {
            map.removeLayer(TripLayerId.legOnOffStops(tripIdx, legIdx));
          }

          if (map.hasLayer(TripLayerId.unselectedLeg(tripIdx, legIdx))) {
            continue;
          }

          const layerId = TripLayerId.unselectedLeg(tripIdx, legIdx);
          map.pushTripLayer(layerId, leg.geometry, leg.paintStyle(false));
          if (legIdx > 0 && !trip.transfersAtStop(legIdx)) {
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
          // Resolved when the line is clicked rather than when it was drawn: a later search
          // replaces the trips, and selecting one looks them up by identity.
          map.on('click', layerId.toString(), () => {
            // Cast for the same reason the list item's handler does: reactive unwrapping loses
            // the class's private members.
            const clicked = this.trips[tripIdx] as Trip | undefined;
            if (clicked) {
              this.clickTrip(clicked);
            }
          });
        }
      }

      // Add selected trip last to be sure it's on top of the unselected trips
      for (let legIdx = 0; legIdx < selectedTrip.legs.length; legIdx++) {
        const leg = selectedTrip.legs[legIdx]!;
        // Pushed before the leg's own layer so the ridden portion draws over it.
        const context = leg.contextLayer();
        if (
          context &&
          !map.hasLayer(TripLayerId.legContext(selectedIdx, legIdx))
        ) {
          map.pushTripLayer(
            TripLayerId.legContext(selectedIdx, legIdx),
            context.geometry,
            context.paint,
          );
        }
        if (!map.hasLayer(TripLayerId.selectedLeg(selectedIdx, legIdx))) {
          map.pushTripLayer(
            TripLayerId.selectedLeg(selectedIdx, legIdx),
            leg.selectedGeometry(),
            leg.paintStyle(true),
          );
        }
        for (const steep of leg.steepSectionLayers()) {
          const steepLayerId = TripLayerId.legSteep(
            selectedIdx,
            legIdx,
            steep.shade,
          );
          if (!map.hasLayer(steepLayerId)) {
            map.pushTripLayer(steepLayerId, steep.geometry, steep.paint);
          }
        }
        // Pushed after both lines, so the dots sit on top of them.
        const contextStops = leg.contextStopsLayer();
        if (
          contextStops &&
          !map.hasLayer(TripLayerId.legContextStops(selectedIdx, legIdx))
        ) {
          map.pushTripStopsLayer(
            TripLayerId.legContextStops(selectedIdx, legIdx),
            contextStops.geometry,
            contextStops.paint,
          );
        }
        const riddenStops = leg.riddenStopsLayer();
        if (
          riddenStops &&
          !map.hasLayer(TripLayerId.legRiddenStops(selectedIdx, legIdx))
        ) {
          map.pushTripStopsLayer(
            TripLayerId.legRiddenStops(selectedIdx, legIdx),
            riddenStops.geometry,
            riddenStops.paint,
          );
        }
        const onOffStops = leg.onOffStopsLayer();
        if (
          onOffStops &&
          !map.hasLayer(TripLayerId.legOnOffStops(selectedIdx, legIdx))
        ) {
          map.pushTripStopsLayer(
            TripLayerId.legOnOffStops(selectedIdx, legIdx),
            onOffStops.geometry,
            onOffStops.paint,
          );
        }
      }
      if (refit) getBaseMap()?.fitBounds(selectedTrip.bounds);

      this.vehicleOverlay?.selectTrip(selectedTrip);
    },
  },
});

function isOnMarker(e: MapMouseEvent): boolean {
  return !!(e.originalEvent.target as Element).closest('.maplibregl-marker');
}

/// `vias` with `via` added where it lengthens the straight path through them least.
function withVia(
  from: LngLat,
  to: LngLat,
  vias: LngLat[],
  via: LngLat,
): LngLat[] {
  const stops = [from, ...vias, to];
  const detours = stops
    .slice(1)
    .map(
      (next, i) =>
        stops[i]!.distanceTo(via) +
        via.distanceTo(next) -
        stops[i]!.distanceTo(next),
    );
  const index = detours.indexOf(Math.min(...detours));
  return [...vias.slice(0, index), via, ...vias.slice(index)];
}
</script>

<style lang="scss">
.search-error {
  padding: 16px;
}

.route-edit-tooltip {
  pointer-events: none;

  .maplibregl-popup-content {
    padding: 4px 8px;
    font-size: 12px;
  }
}
</style>
