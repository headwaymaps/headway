<template>
  <div class="top-card">
    <trip-search
      :from-place="fromPlace"
      :to-place="toPlace"
      :current-mode="mode"
      directions-path="/compare-directions"
      :initial-search="transitQuery.params"
      :did-select-from-place="searchBoxDidSelectFromPlace"
      :did-select-to-place="searchBoxDidSelectToPlace"
      :did-swap-places="clickedSwap"
      :search-did-change="searchDidChange"
    />
  </div>
  <div class="bottom-card">
    <trip-corpus
      v-if="config"
      :url="config.corpus"
      :from="fromPlace?.point"
      :to="toPlace?.point"
      @select="selectCorpusTrip"
    />
    <route-comparison-card
      v-if="config && comparison"
      :routers="config.routers"
      :routes="routes as ComparedRoute[]"
      :comparison="comparison"
      :fitting-differences="fittingDifferences"
      @select="renderRoutes"
      @toggle-fit="toggleFit"
    />
    <p v-if="error" class="search-error">
      {{ error.message ?? 'Routing failed.' }}
    </p>
    <q-linear-progress v-if="isLoading" indeterminate />
    <q-list v-if="comparison">
      <trip-list-item
        v-for="(route, i) in routes"
        :key="i"
        :click-handler="() => renderRoutes(i)"
        :active="i === selectedIdx"
        :duration-formatted="route.trip.durationFormatted"
        :distance-formatted="route.trip.distanceFormatted"
      >
        <span
          class="route-letter"
          :style="{ background: comparison.routes[i]?.color }"
          >{{ comparison.routes[i]?.letter }}</span
        >
        <component
          :is="componentForMode(route.trip.mode)"
          :trip="route.trip"
          :active="i === selectedIdx"
        />
      </trip-list-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import { getBaseMap, map as rawMap } from 'src/components/BaseMap.vue';
import { Component, defineComponent, markRaw, Ref, ref } from 'vue';
import { useRoute } from 'vue-router';
import { LngLat } from 'maplibre-gl';
import Place, { PlaceId, PlaceStorage } from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import TripListItem from 'src/components/TripListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import RouteComparisonCard from 'src/components/RouteComparisonCard.vue';
import TripCorpus from 'src/components/TripCorpus.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';
import RouteComparison from 'src/models/RouteComparison';
import ComparedRoutes, {
  ComparedRoute,
  ComparedRouter,
} from 'src/models/ComparedRoutes';
import { TripFetchError } from 'src/models/Trip';
import { TravelmuxClient, TravelmuxMode } from 'src/services/TravelmuxClient';
import Prefs from 'src/utils/Prefs';
import Markers from 'src/utils/Markers';
import TransitQuery, { TransitQueryParams } from 'src/models/TransitQuery';

/// contrib/otp-compare's compare.json.
type CompareConfig = {
  routers: ComparedRouter[];
  /// Where the trip corpus is served.
  corpus: string;
};

/// Compares the routes several router builds plan for a trip.
export default defineComponent({
  name: 'ComparePage',
  components: { TripListItem, TripSearch, RouteComparisonCard, TripCorpus },
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
    const transitQuery: Ref<TransitQuery> = ref(
      TransitQuery.parseFromQuery(useRoute().query),
    );
    return { toPlace, fromPlace, transitQuery };
  },
  data(): {
    config?: CompareConfig;
    routes: ComparedRoute[];
    selectedIdx: number;
    error?: TripFetchError;
    isLoading: boolean;
    /// Keeps stepping through routes on their differences rather than the whole trips.
    fittingDifferences: boolean;
    /// Counts route requests, so a slow one doesn't replace the routes of a later one.
    latestRequest: number;
  } {
    return {
      config: undefined,
      routes: [],
      selectedIdx: 0,
      error: undefined,
      isLoading: false,
      fittingDifferences: true,
      latestRequest: 0,
    };
  },
  computed: {
    comparison(): RouteComparison | undefined {
      if (this.routes.length === 0) return undefined;
      return markRaw(
        new RouteComparison(
          this.routes.map((route) =>
            route.trip.legs.flatMap((leg) => leg.geometry.coordinates),
          ),
        ),
      );
    },
  },
  watch: {
    mode: async function (): Promise<void> {
      await this.updateRoutes();
    },
  },
  mounted: async function () {
    // Shown as soon as it arrives, rather than once the places have been looked up.
    const config = fetch('/compare.json')
      .then((r) => r.json())
      .then((config: CompareConfig) => (this.config = config));
    [this.fromPlace, this.toPlace] = await Promise.all(
      [this.from, this.to].map((id) =>
        id == '_' ? undefined : PlaceStorage.fetchFromSerializedId(id),
      ),
    );
    await config;
    await this.rewriteUrl();
  },
  methods: {
    componentForMode(mode: TravelMode): Component {
      return mode === TravelMode.Transit
        ? MultiModalListItem
        : SingleModeListItem;
    },
    /// The selected mode in travelmux's terms, with cycling added for transit with a bike.
    travelmuxModes(): TravelmuxMode[] {
      switch (this.mode) {
        case TravelMode.Walk:
          return [TravelmuxMode.Walk];
        case TravelMode.Bike:
          return [TravelmuxMode.Bike];
        case TravelMode.Drive:
          return [TravelmuxMode.Drive];
        case TravelMode.Transit:
          return this.transitQuery.params.transitWithBicycle
            ? [TravelmuxMode.Transit, TravelmuxMode.Bike]
            : [TravelmuxMode.Transit];
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
    clickedSwap(newFromValue?: Place, newToValue?: Place) {
      this.fromPlace = newFromValue;
      this.toPlace = newToValue;
      this.rewriteUrl();
    },
    searchDidChange(newValue: TransitQueryParams) {
      this.transitQuery = new TransitQuery(newValue);
      this.rewriteUrl();
    },
    /// Routes between the bare points right away, then names them once the geocoder answers.
    async selectCorpusTrip(from: LngLat, to: LngLat) {
      [this.fromPlace, this.toPlace] = [from, to].map(Place.bareLocation);
      const named = Promise.all(
        [from, to].map((p) => PlaceStorage.fetchFromId(PlaceId.location(p))),
      );
      await this.rewriteUrl();
      const [namedFrom, namedTo] = await named;
      const unchanged = (place?: Place, named?: Place) =>
        place?.serializedId() === named?.serializedId();
      if (
        unchanged(this.fromPlace, namedFrom) &&
        unchanged(this.toPlace, namedTo)
      ) {
        [this.fromPlace, this.toPlace] = [namedFrom, namedTo];
      }
    },
    async rewriteUrl() {
      if (!this.fromPlace && !this.toPlace) {
        this.$router.push('/');
        return;
      }
      const fromEncoded = this.fromPlace?.urlEncodedId() ?? '_';
      const toEncoded = this.toPlace?.urlEncodedId() ?? '_';
      this.$router.push({
        path: `/compare-directions/${this.mode}/${toEncoded}/${fromEncoded}`,
        query: this.transitQuery.searchQuery(),
      });
      await this.updateRoutes();
    },
    async updateRoutes() {
      const map = getBaseMap();
      if (!map || !this.config) return;
      const request = ++this.latestRequest;
      map.removeAllLayers();
      map.removeAllMarkers();
      this.routes = [];
      this.error = undefined;
      this.isLoading = false;

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

      if (this.fromPlace && this.toPlace) {
        const [from, to] = [this.fromPlace, this.toPlace];
        const params = this.transitQuery.params;
        this.isLoading = true;
        const routers = this.config.routers;
        const result = await TravelmuxClient.fetchPlans(
          from.point,
          to.point,
          this.travelmuxModes(),
          5,
          Prefs.stored.distanceUnits(from, to),
          params.searchTime,
          params.searchDate,
          params.arriveBy,
          routers.map(({ routerId }) => routerId),
        );
        if (request !== this.latestRequest) return;
        this.isLoading = false;
        if (!result.ok) {
          this.error = result.error;
          return;
        }

        const compared = new ComparedRoutes();
        for (const { name, routerId } of routers) {
          compared.add(
            name,
            result.value.filter((trip) => trip.raw.routerId === routerId),
          );
        }
        this.routes = markRaw(compared.routes);
        if (this.routes.length > 0) {
          this.error = undefined;
          this.renderRoutes(0);
        }
      }
    },
    fitRoute() {
      const route = this.routes[this.selectedIdx];
      if (!route) return;
      const differences =
        this.fittingDifferences && this.comparison?.differenceBounds;
      getBaseMap()?.fitBounds(
        differences || route.trip.bounds,
        differences ? { maxZoom: 17 } : undefined,
      );
    },
    toggleFit() {
      this.fittingDifferences = !this.fittingDifferences;
      this.fitRoute();
    },
    renderRoutes(selectedIdx: number) {
      const map = getBaseMap();
      if (!map || !rawMap) return;
      this.selectedIdx = selectedIdx;
      this.comparison?.draw(map, rawMap, selectedIdx);
      this.fitRoute();
    },
  },
});
</script>

<style lang="scss">
.search-error {
  padding: 16px;
}
</style>
