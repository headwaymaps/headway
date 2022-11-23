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
      <route-list-item
        v-for="item in $data.routes"
        :click-handler="() => clickRoute(item)"
        :active="$data.activeRoute === item"
        :duration-formatted="item.durationFormatted"
        :distance-formatted="item.lengthFormatted"
        v-bind:key="JSON.stringify(item)"
      >
        <component
          :is="componentForMode(mode)"
          :item="item"
          :showRouteSteps="showRouteSteps"
        />
      </route-list-item>
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
  POI,
  poiDisplayName,
} from 'src/utils/models';
import { Component, defineComponent, Ref, ref } from 'vue';
import { LngLat, LngLatBounds } from 'maplibre-gl';
import { CacheableMode } from 'src/services/ValhallaClient';
import Route from 'src/models/Route';
import Place from 'src/models/Place';
import { TravelMode } from 'src/utils/models';
import RouteListItem from 'src/components/RouteListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import SingleModeListItem from 'src/components/SingleModeListItem.vue';

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
    routes: Route[];
    activeRoute: Route | undefined;
  } {
    return {
      routes: [],
      activeRoute: undefined,
    };
  },
  components: { RouteListItem, TripSearch },
  methods: {
    componentForMode(mode: TravelMode): Component {
      switch (mode) {
        case TravelMode.Walk:
        case TravelMode.Bike:
        case TravelMode.Drive:
          return SingleModeListItem;
        case TravelMode.Transit:
          throw 'todo';
      }
    },
    poiDisplayName,
    clickRoute(route: Route) {
      this.$data.activeRoute = route;
      let index = this.$data.routes.indexOf(route);
      if (index !== -1) {
        this.renderRoutes(this.$data.routes, index);
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
    showRouteSteps(route: Route) {
      console.assert(
        this.mode != TravelMode.Transit,
        'show route steps should only be availble for non-transit'
      );
      let index = this.$data.routes.indexOf(route);
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
      await this.updateRoutes();
    },

    async updateRoutes(): Promise<void> {
      getBaseMap()?.removeAllLayers();
      getBaseMap()?.removeAllMarkers();
      if (fromPoi.value?.position && toPoi.value?.position) {
        const fromCanonical = canonicalizePoi(fromPoi.value);
        // TODO: replace POI with Place so we don't have to hit pelias twice?
        let fromPlace = await Place.fetchFromSerializedId(fromCanonical);
        const routes = await Route.getRoutes(
          fromPoi.value,
          toPoi.value,
          this.mode as CacheableMode,
          fromPlace.preferredDistanceUnits()
        );
        this.renderRoutes(routes, 0);
      }
    },
    renderRoutes(routes: Route[], selectedIdx: number) {
      const map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly empty');
        return;
      }
      this.$data.routes = routes;
      this.activeRoute = routes[selectedIdx];

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

      const unselectedLayerName = (routeIdx: number) =>
        `aleternate_${this.mode}_${routeIdx}_unselected`;
      const selectedLayerName = (routeIdx: number) =>
        `aleternate_${this.mode}_${routeIdx}_selected`;

      for (let routeIdx = 0; routeIdx < routes.length; routeIdx++) {
        if (routeIdx == selectedIdx) {
          if (map.hasLayer(unselectedLayerName(routeIdx))) {
            map.removeLayer(unselectedLayerName(routeIdx));
          }
          continue;
        }

        if (map.hasLayer(selectedLayerName(routeIdx))) {
          map.removeLayer(selectedLayerName(routeIdx));
        }

        if (map.hasLayer(unselectedLayerName(routeIdx))) {
          continue;
        }

        const route = routes[routeIdx];
        map.pushRouteLayer(unselectedLayerName(routeIdx), route.geometry(), {
          'line-color': '#777',
          'line-width': 4,
          'line-dasharray': [0.5, 2],
        });
      }

      const selectedRoute = routes[selectedIdx];
      if (!map.hasLayer(selectedLayerName(selectedIdx))) {
        // Add selected route last to be sure it's on top of the unselected routes
        map.pushRouteLayer(
          selectedLayerName(selectedIdx),
          selectedRoute.geometry(),
          {
            'line-color': '#1976D2',
            'line-width': 6,
          }
        );
      }
      setTimeout(async () => {
        this.resizeMap();
      });
      const summary = selectedRoute.valhallaRoute.summary;
      getBaseMap()?.fitBounds(
        new LngLatBounds(
          new LngLat(summary.min_lon, summary.min_lat),
          new LngLat(summary.max_lon, summary.max_lat)
        )
      );
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
  },
  watch: {
    mode: async function (): Promise<void> {
      await this.updateRoutes();
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
