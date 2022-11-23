<template>
  <trip-search
    :from-poi="fromPoi"
    :to-poi="toPoi"
    :current-mode="transitMode"
    :did-select-from-poi="searchBoxDidSelectFromPoi"
    :did-select-to-poi="searchBoxDidSelectToPoi"
    :did-swap-pois="clickedSwap"
  />
  <div class="bottom-card bg-white" ref="bottomCard" v-if="fromPoi && toPoi">
    <q-list>
      <route-list-item
        v-for="(item, index) in itineraries"
        :click-handler="() => changeItinerary(index)"
        :active="$data.itineraryIndex === index"
        :duration-formatted="item.durationFormatted()"
        distance-formatted=""
        v-bind:key="JSON.stringify(item)"
      >
        <component
          :is="componentForMode('transit')"
          :item="item"
          :active="index === itineraryIndex"
          :earliest-start="earliestStart"
          :latest-arrival="latestArrival"
        />
      </route-list-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import {
  destinationMarker,
  getBaseMap,
  setBottomCardAllowance,
  sourceMarker,
} from 'src/components/BaseMap.vue';
import {
  encodePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
  TravelMode,
} from 'src/utils/models';
import { Component, defineComponent, Ref, ref } from 'vue';
import TransitTimeline from 'src/components/TransitTimeline.vue';
import RouteListItem from 'src/components/RouteListItem.vue';
import TripSearch from 'src/components/TripSearch.vue';
import Itinerary from 'src/models/Itinerary';
import { toLngLat } from 'src/utils/geomath';
import { DistanceUnits } from 'src/utils/models';
import MultiModalListItem from 'src/components/MultiModalListItem.vue';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'TransitPage',
  props: {
    to: String,
    from: String,
  },
  data: function (): {
    itineraries: Itinerary[];
    itineraryIndex: number;
    earliestStart: number;
    latestArrival: number;
    visibleSteps: { [key: number]: boolean };
  } {
    return {
      itineraries: [],
      itineraryIndex: 0,
      earliestStart: 0,
      latestArrival: 0,
      visibleSteps: {},
    };
  },
  components: { RouteListItem, TripSearch, TransitTimeline },
  methods: {
    showSteps(index: number) {
      this.$data.visibleSteps[index] = true;
    },
    areStepsVisible(index: number): boolean {
      return this.$data.visibleSteps[index] === true;
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    componentForMode(mode: 'transit'): Component {
      return MultiModalListItem;
    },
    poiDisplayName,
    changeItinerary(index: number) {
      this.$data.itineraryIndex = index;
      this.plotPaths();
    },
    clickedSwap(newFromValue?: POI, newToValue?: POI) {
      fromPoi.value = newFromValue;
      toPoi.value = newToValue;
      this.rewriteUrl();
    },
    searchBoxDidSelectFromPoi(poi?: POI) {
      this.fromPoi = poi;
      this.rewriteUrl();
    },
    searchBoxDidSelectToPoi(poi?: POI) {
      this.toPoi = poi;
      this.rewriteUrl();
    },
    rewriteUrl: async function () {
      let map = getBaseMap();
      if (!map) {
        console.error('basemap was unexpectedly empty');
        return;
      }
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }

      if (fromPoi.value?.position) {
        map.pushMarker(
          'source_marker',
          sourceMarker().setLngLat([
            fromPoi.value.position.long,
            fromPoi.value.position.lat,
          ])
        );
      } else {
        map.removeMarker('source_marker');
      }
      if (toPoi.value?.position) {
        map.pushMarker(
          'destination_marker',
          destinationMarker().setLngLat([
            toPoi.value.position.long,
            toPoi.value.position.lat,
          ])
        );
      } else {
        map.removeMarker('destination_marker');
      }
      const fromCanonical = fromPoi.value ? encodePoi(fromPoi.value) : '_';
      const toCanonical = toPoi.value ? encodePoi(toPoi.value) : '_';
      this.$router.push(`/multimodal/${toCanonical}/${fromCanonical}`);
      if (fromPoi.value?.position && toPoi.value?.position) {
        this.$data.itineraries = await Itinerary.fetchBest(
          toLngLat(fromPoi.value.position),
          toLngLat(toPoi.value.position),
          DistanceUnits.Miles
        );
        this.calculateStats();
        this.plotPaths();
      } else {
        map.removeAllLayers();
      }
    },
    calculateStats() {
      this.$data.earliestStart = Number.MAX_SAFE_INTEGER;
      this.$data.latestArrival = 0;
      for (var index = 0; index < this.$data.itineraries.length; index++) {
        this.$data.earliestStart = Math.min(
          this.$data.earliestStart,
          this.$data.itineraries[index].startTime
        );
        this.$data.latestArrival = Math.max(
          this.$data.latestArrival,
          this.$data.itineraries[index].endTime
        );
      }
    },
    plotPaths() {
      // TODO: avoid flicker when switching between selected like
      // we do in alternates
      getBaseMap()?.removeAllLayers();
      for (let i = 0; i < this.$data.itineraries.length; i++) {
        if (i == this.$data.itineraryIndex) {
          // plot the selected one last
          continue;
        }
        this.plotPath(this.$data.itineraries[i], i, false);
      }
      const itinerary = this.$data.itineraries[this.$data.itineraryIndex];
      this.plotPath(itinerary, this.$data.itineraryIndex, true);
    },
    plotPath(itinerary: Itinerary, route_idx: number, active: boolean) {
      for (const legIdx in itinerary.legs) {
        let leg = itinerary.legs[legIdx];
        const layerName = `headway_transit_route_${route_idx}_leg_${legIdx}`;
        getBaseMap()?.pushRouteLayer(
          layerName,
          leg.geometry(),
          leg.paintStyle(active)
        );
      }
      getBaseMap()?.fitBounds(itinerary.bounds);
    },
    resizeMap() {
      // TODO: this impl copied from AlternatesPage.vue. I'm not sure if its correct
      // but what was before was erroring due to referencing non-existant members.
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
    // NOTE: this doesn't seem to be called
    to(newValue) {
      setTimeout(async () => {
        toPoi.value = await decanonicalizePoi(newValue);
        await this.rewriteUrl();
        this.resizeMap();
      });
    },
    // NOTE: this doesn't seem to be called
    from(newValue) {
      setTimeout(async () => {
        fromPoi.value = await decanonicalizePoi(newValue);
        await this.rewriteUrl();
        this.resizeMap();
      });
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      toPoi.value = await decanonicalizePoi(this.$props.to as string);
      fromPoi.value = await decanonicalizePoi(this.$props.from as string);
      await this.rewriteUrl();
      this.resizeMap();
    });
  },
  unmounted: function () {
    getBaseMap()?.removeLayersExcept([]);
  },
  setup: function () {
    return {
      toPoi,
      fromPoi,
      transitMode: TravelMode.Transit,
    };
  },
});
</script>
