<template>
  <div class="top-left-card">
    <q-card>
      <q-card-section>
        <div :style="{ display: 'flex', alignItems: 'center' }">
          <search-box
            ref="searchBox"
            :hint="$t('search.from')"
            :style="{ flex: 1 }"
            :force-text="fromPoi ? poiDisplayName(fromPoi) : undefined"
            v-on:did-select-poi="searchBoxDidSelectFromPoi"
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
          :force-text="toPoi ? poiDisplayName(toPoi) : undefined"
          v-on:did-select-poi="searchBoxDidSelectToPoi"
        ></search-box>
      </q-card-section>
    </q-card>
  </div>
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
        <q-item-label>
          {{ item.startStopTimesFormatted() }}
        </q-item-label>
        <q-item-label caption>
          {{ item.viaRouteFormatted }}
        </q-item-label>
        <q-item-label caption>
          {{
            $t('walk_distance', {
              preformattedDistance: item.walkingDistanceFormatted(),
            })
          }}
        </q-item-label>
        <q-item-label
          :hidden="$data.itineraryIndex === index && areStepsVisible(index)"
        >
          <q-btn
            style="margin-left: -6px"
            padding="6px"
            flat
            icon="directions"
            label="Details"
            size="sm"
            v-on:click="showSteps(index)"
          />
        </q-item-label>
        <transit-timeline
          :hidden="!($data.itineraryIndex === index && areStepsVisible(index))"
          :itinerary="item"
          :earliest-start="earliestStart"
          :latest-arrival="latestArrival"
        />
      </route-list-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import { i18n } from 'src/i18n/lang';
import {
  getBaseMap,
  map,
  setBottomCardAllowance,
} from 'src/components/BaseMap.vue';
import {
  encodePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/utils/models';
import { defineComponent, Ref, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';
import TransitTimeline from 'src/components/TransitTimeline.vue';
import RouteListItem from 'src/components/RouteListItem.vue';
import { decodeOtpPath } from 'src/third_party/decodePath';
import { LngLatBounds, Marker } from 'maplibre-gl';
import Itinerary from 'src/models/Itinerary';
import { useQuasar } from 'quasar';
import { toLngLat } from 'src/utils/geomath';
import { DistanceUnits } from 'src/utils/models';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'TransitPage',
  props: {
    mode: String,
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
  components: { SearchBox, RouteListItem, TransitTimeline },
  methods: {
    showSteps(index: number) {
      this.$data.visibleSteps[index] = true;
    },
    areStepsVisible(index: number): boolean {
      return this.$data.visibleSteps[index] === true;
    },
    poiDisplayName,
    changeItinerary(index: number) {
      this.$data.itineraryIndex = index;
      this.plotPaths();
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
    searchBoxDidSelectFromPoi(poi?: POI) {
      this.fromPoi = poi;
      this.rewriteUrl();
    },
    searchBoxDidSelectToPoi(poi?: POI) {
      this.toPoi = poi;
      this.rewriteUrl();
    },
    rewriteUrl: async function () {
      if (!fromPoi.value?.position && !toPoi.value?.position) {
        this.$router.push('/');
        return;
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
        // FIXME: Here and below, don't hardcode a credible maximum number of legs, it's very silly, just store it.
        for (var index = 0; index < 100; index++) {
          const layerName = `headway_polyline${index}`;
          if (map?.getLayer(layerName)) {
            map?.removeLayer(layerName);
          }
          if (map?.getSource(layerName)) {
            map?.removeSource(layerName);
          }
        }
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
      const bbox = new LngLatBounds();
      for (var leg_idx in itinerary.legs) {
        const leg = itinerary.legs[leg_idx];
        const layerName = `headway_transit_route_${route_idx}_leg_${leg_idx}`;
        const points: [number, number][] = decodeOtpPath(
          leg.legGeometry.points
        );
        for (const lngLat of points) {
          bbox.extend(lngLat);
        }
        getBaseMap()?.pushLayer(
          layerName,
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
            id: layerName,
            type: 'line',
            source: layerName,
            layout: {
              'line-join': 'round',
              'line-cap': 'round',
            },
            paint: {
              'line-color': active
                ? leg.transitLeg
                  ? '#E21919'
                  : '#1976D2'
                : '#777',
              'line-width': leg.transitLeg ? 6 : 4,
              'line-dasharray': leg.transitLeg ? [1] : [1, 2],
            },
          },
          'symbol'
        );
      }
      getBaseMap()?.fitBounds(bbox);
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

      if (this.toPoi?.position) {
        getBaseMap()?.pushMarker(
          'active_marker',
          new Marker({ color: '#111111' }).setLngLat([
            this.toPoi.position.long,
            this.toPoi.position.lat,
          ])
        );
        getBaseMap()?.removeMarkersExcept(['active_marker']);
      }
    });
  },
  unmounted: function () {
    getBaseMap()?.removeLayersExcept([]);
  },
  setup: function () {
    return {
      toPoi,
      fromPoi,
    };
  },
});
</script>
