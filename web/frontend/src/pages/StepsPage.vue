<template>
  <div class="top-left-card"></div>
  <q-btn
    round
    icon="arrow_back"
    class="top-left-fab"
    v-on:click="() => onBackClicked()"
  />
  <div class="bottom-card bg-white" ref="bottomCard" v-if="fromPoi && toPoi">
    <q-list>
      <div v-for="item in $data.steps" v-bind:key="JSON.stringify(item)">
        <q-item class="q-my-sm" active-class="bg-blue-1">
          <q-item-section avatar>
            <q-icon :name="valhallaTypeToIcon(item.type)" />
          </q-item-section>
          <q-item-section>
            <q-item-label>
              {{ item.instruction }}
            </q-item-label>
            <q-item-label caption>
              {{ item.verbal_post_transition_instruction }}
            </q-item-label>
          </q-item-section>
        </q-item>
        <q-separator spaced />
      </div>
    </q-list>
  </div>
</template>

<script lang="ts">
import {
  getBaseMap,
  map,
  setBottomCardAllowance,
} from 'src/components/BaseMap.vue';
import {
  encodePoi,
  canonicalizePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/utils/models';
import Place from 'src/models/Place';
import Route from 'src/models/Route';
import { defineComponent, Ref, ref } from 'vue';
import { decodeValhallaPath } from 'src/third_party/decodePath';
import { LngLat, LngLatBounds, Marker } from 'maplibre-gl';
import {
  ValhallaRouteLegManeuver,
  CacheableMode,
  getRoutes,
  ValhallaRoute,
  summarizeRoute,
  valhallaTypeToIcon,
} from 'src/services/ValhallaClient';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'StepsPage',
  props: {
    mode: String,
    to: String,
    from: String,
    alternateIndex: String,
  },
  data: function (): {
    steps: ValhallaRouteLegManeuver[];
  } {
    return {
      steps: [],
    };
  },
  methods: {
    poiDisplayName,
    summarizeRoute,
    valhallaTypeToIcon,
    onBackClicked() {
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }
      const fromCanonical = fromPoi.value ? encodePoi(fromPoi.value) : '_';
      const toCanonical = toPoi.value ? encodePoi(toPoi.value) : '_';
      this.$router.push(
        `/directions/${this.mode}/${toCanonical}/${fromCanonical}`
      );
    },
    rewriteUrl: async function () {
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }
      const fromCanonical = fromPoi.value
        ? canonicalizePoi(fromPoi.value)
        : '_';
      const toEncoded = toPoi.value ? encodePoi(toPoi.value) : '_';
      this.$router.push(
        `/directions/${this.mode}/${toEncoded}/${encodeURIComponent(
          fromCanonical
        )}/${this.alternateIndex}`
      );
      if (fromPoi.value?.position && toPoi.value?.position) {
        // TODO: replace POI with Place so we don't have to hit pelias twice?
        let fromPlace = await Place.fetchFromSerializedId(fromCanonical);
        const routes = await getRoutes(
          fromPoi.value,
          toPoi.value,
          this.mode as CacheableMode,
          fromPlace.preferredDistanceUnits()
        );
        if (this.alternateIndex) {
          let idx = parseInt(this.alternateIndex);
          this.processRoute(routes, idx);
        }
      } else {
        if (map?.getLayer('headway_polyline')) {
          map?.removeLayer('headway_polyline');
        }
        if (map?.getSource('headway_polyline')) {
          map?.removeSource('headway_polyline');
        }
      }
    },
    processRoute(routes: [ValhallaRoute, Route][], selectedIdx: number) {
      for (var i = 0; i < 10; i += 1) {
        if (map?.getLayer('headway_polyline' + i)) {
          map?.removeLayer('headway_polyline' + i);
        }
        if (map?.getSource('headway_polyline' + i)) {
          map?.removeSource('headway_polyline' + i);
        }
      }
      const route = routes[selectedIdx];
      const leg = route[0]?.legs[0];
      this.$data.steps = leg.maneuvers;
      if (leg && map) {
        var totalTime = 0;
        for (const key in leg.maneuvers) {
          totalTime += leg.maneuvers[key].time;
          leg.maneuvers[key].time = totalTime;
        }
        var points: [number, number][] = [];
        decodeValhallaPath(leg.shape, 6).forEach((point) => {
          points.push([point[1], point[0]]);
        });
        getBaseMap()?.pushRouteLayer(leg, 'headway_polyline' + selectedIdx, {
          'line-color': '#1976D2',
          'line-width': 6,
        });
        setTimeout(() => {
          this.resizeMap();
          getBaseMap()?.fitBounds(
            new LngLatBounds(
              new LngLat(route[0].summary.min_lon, route[0].summary.min_lat),
              new LngLat(route[0].summary.max_lon, route[0].summary.max_lat)
            )
          );
        });
      }
    },
    clearPolylines() {
      for (var i = 0; i < 10; i += 1) {
        if (map?.getLayer('headway_polyline' + i)) {
          map?.removeLayer('headway_polyline' + i);
        }
        if (map?.getSource('headway_polyline' + i)) {
          map?.removeSource('headway_polyline' + i);
        }
      }
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
    to(newValue) {
      setTimeout(async () => {
        toPoi.value = await decanonicalizePoi(newValue);
        if (!toPoi.value) {
          this.clearPolylines();
        }
        this.resizeMap();

        getBaseMap()?.removeMarkersExcept([]);

        if (!newValue.position) {
          return;
        }
        const marker = new Marker({ color: '#111111' }).setLngLat([
          newValue.position.long,
          newValue.position.lat,
        ]);
        getBaseMap()?.pushMarker('active_marker', marker);
      });
    },
    from(newValue) {
      setTimeout(async () => {
        fromPoi.value = await decanonicalizePoi(newValue);
        if (!fromPoi.value) {
          this.clearPolylines();
        }
        this.resizeMap();
      });
    },
  },
  beforeUnmount: function () {
    this.clearPolylines();
  },
  mounted: async function () {
    setTimeout(async () => {
      toPoi.value = await decanonicalizePoi(this.$props.to as string);
      fromPoi.value = await decanonicalizePoi(this.$props.from as string);
      await this.rewriteUrl();
      this.resizeMap();

      getBaseMap()?.removeMarkersExcept([]);
      if (this.toPoi?.position) {
        const marker = new Marker({ color: '#111111' }).setLngLat([
          this.toPoi.position.long,
          this.toPoi.position.lat,
        ]);
        getBaseMap()?.pushMarker('active_marker', marker);
      }
    });
  },
  unmounted: function () {
    if (map?.getLayer('headway_polyline')) {
      map?.removeLayer('headway_polyline');
    }
    if (map?.getSource('headway_polyline')) {
      map?.removeSource('headway_polyline');
    }
  },
  setup: function () {
    return {
      toPoi,
      fromPoi,
    };
  },
});
</script>
