<template>
  <div class="top-left-card">
    <q-card>
      <q-card-section>
        <div :style="{ display: 'flex', alignItems: 'center' }">
          <search-box
            ref="searchBox"
            :hint="$t('search.from')"
            :style="{ flex: 1 }"
            v-model="fromPoi"
            :force-text="fromPoi ? poiDisplayName(fromPoi) : undefined"
            v-on:update:model-value="rewriteUrl"
          >
          </search-box>
          <q-btn
            size="small"
            :style="{ marginLeft: '0.5em', marginRight: 0 }"
            flat
            round
            color="primary"
            icon="gps_fixed"
            v-on:click="fromUserLocation"
          />
        </div>
      </q-card-section>
      <q-card-section class="no-top-padding">
        <search-box
          ref="searchBox"
          :hint="$t('search.to')"
          v-model="toPoi"
          :force-text="toPoi ? poiDisplayName(toPoi) : undefined"
          v-on:update:model-value="rewriteUrl"
        ></search-box>
      </q-card-section>
    </q-card>
  </div>
  <div class="bottom-card bg-white" ref="bottomCard" v-if="fromPoi && toPoi">
    <q-list>
      <div v-for="item in $data.routes" v-bind:key="JSON.stringify(item)">
        <q-item
          class="q-my-sm"
          clickable
          v-ripple
          :active="$data.activeRoute === item"
          v-on:click="clickRoute(item)"
          active-class="bg-blue-1"
        >
          <q-item-section>
            <q-item-label>
              {{ item[1].timeFormatted }}
              <span class="text-weight-light">{{
                ' (' + item[1].lengthFormatted + ')'
              }}</span>
            </q-item-label>
            <q-item-label caption v-if="item[1].viaRoadsFormatted.length !== 0">
              {{ $t('via_$place', { place: item[1].viaRoadsFormatted }) }}
            </q-item-label>
          </q-item-section>

          <q-item-section side>
            <q-icon
              name="directions"
              v-on:click="showSteps(item)"
              :color="item === $data.activeRoute ? 'blue' : ''"
            />
          </q-item-section>
        </q-item>
        <q-separator spaced />
      </div>
    </q-list>
  </div>
</template>

<script lang="ts">
import { getBaseMap, setBottomCardAllowance } from 'src/components/BaseMap.vue';
import {
  encodePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/utils/models';
import { defineComponent, Ref, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';
import { decodeValhallaPath } from 'src/third_party/decodePath';
import { LngLat, LngLatBounds, Marker } from 'maplibre-gl';
import { useQuasar } from 'quasar';
import { CacheableMode, getRoutes } from 'src/utils/routecache';
import { Route, ProcessedRouteSummary, summarizeRoute } from 'src/utils/routes';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'AlternatesPage',
  props: {
    mode: String,
    to: String,
    from: String,
  },
  data: function (): {
    routes: [Route, ProcessedRouteSummary][];
    activeRoute: [Route, ProcessedRouteSummary] | undefined;
    points: [number, number][];
  } {
    return {
      routes: [],
      activeRoute: undefined,
      points: [],
    };
  },
  components: { SearchBox },
  methods: {
    poiDisplayName,
    summarizeRoute,
    clickRoute(route: [Route, ProcessedRouteSummary]) {
      this.$data.activeRoute = route;
      let index = this.$data.routes.indexOf(route);
      if (index !== -1) {
        this.processRoute(this.$data.routes, index);
      }
    },
    showSteps(route: [Route, ProcessedRouteSummary]) {
      let index = this.$data.routes.indexOf(route);
      if (index !== -1 && this.to && this.from) {
        this.$router.push(
          `/directions/${this.mode}/${encodeURIComponent(
            this.to
          )}/${encodeURIComponent(this.from)}/${index}`
        );
      }
    },
    fromUserLocation() {
      const options = {
        enableHighAccuracy: true,
        maximumAge: 60000,
        timeout: 10000,
      };
      navigator.geolocation.getCurrentPosition(
        (position) => {
          fromPoi.value = {
            name: this.$t('my_location'),
            position: {
              lat: position.coords.latitude,
              long: position.coords.longitude,
            },
          };
          setTimeout(async () => {
            await this.rewriteUrl();
          });
        },
        (error) => {
          useQuasar().notify(this.$t('could_not_get_gps_location'));
          console.error(error);
        },
        options
      );
    },
    rewriteUrl: async function () {
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
      }
      const fromCanonical = fromPoi.value ? encodePoi(fromPoi.value) : '_';
      const toCanonical = toPoi.value ? encodePoi(toPoi.value) : '_';
      this.$router.push(
        `/directions/${this.mode}/${toCanonical}/${fromCanonical}`
      );
      if (fromPoi.value?.position && toPoi.value?.position) {
        const routes = await getRoutes(
          fromPoi.value,
          toPoi.value,
          this.mode as CacheableMode
        );
        this.processRoute(routes, 0);
      } else {
        getBaseMap()?.removeLayersExcept([]);
        getBaseMap()?.removeMarkersExcept([]);
      }
    },
    processRoute(
      routes: [Route, ProcessedRouteSummary][],
      selectedIdx: number
    ) {
      this.$data.routes = routes;
      this.activeRoute = routes[selectedIdx];

      let activeLayers = [];
      getBaseMap()?.removeLayersExcept([]);
      for (let routeIdx = 0; routeIdx < routes.length; routeIdx += 1) {
        const route = routes[routeIdx];
        const leg = route[0]?.legs[0];
        if (leg) {
          var totalTime = 0;
          for (const key in leg.maneuvers) {
            totalTime += leg.maneuvers[key].time;
            leg.maneuvers[key].time = totalTime;
          }
          var points: [number, number][] = [];
          decodeValhallaPath(leg.shape, 6).forEach((point) => {
            points.push([point[1], point[0]]);
          });
          this.$data.points = points;

          let polylineKey = 'headway_polyline' + routeIdx;
          activeLayers.push(polylineKey);
          getBaseMap()?.pushLayer(
            polylineKey,
            {
              type: 'geojson',
              data: {
                type: 'Feature',
                properties: {},
                geometry: {
                  type: 'LineString',
                  coordinates: points,
                },
              },
            },
            {
              id: 'headway_polyline' + routeIdx,
              type: 'line',
              source: 'headway_polyline' + routeIdx,
              layout: {
                'line-join': 'round',
                'line-cap': 'round',
              },
              paint:
                routeIdx === selectedIdx
                  ? {
                      'line-color': '#1976D2',
                      'line-width': 6,
                    }
                  : {
                      'line-color': '#1976D2',
                      'line-width': 4,
                      'line-dasharray': [1, 2],
                    },
            }
          );
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
        this.resizeMap();

        if (!newValue.position) {
          return;
        }
        getBaseMap()?.pushMarker(
          'active_marker',
          new Marker({ color: '#111111' }).setLngLat([
            newValue.position.long,
            newValue.position.lat,
          ])
        );
        getBaseMap()?.removeMarkersExcept(['active_marker']);
      });
    },
    from(newValue) {
      setTimeout(async () => {
        fromPoi.value = await decanonicalizePoi(newValue);
        this.resizeMap();
      });
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
  setup: function () {
    return {
      toPoi,
      fromPoi,
    };
  },
});
</script>
