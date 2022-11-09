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
            :style="{ marginRight: '0.8em' }"
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
  <div class="bottom-card" ref="bottomCard" v-if="fromPoi && toPoi">
    <q-card>
      <q-card-section class="bg-primary text-white">
        <ul class="itinerary-container">
          <li
            :class="
              index == itineraryIndex
                ? 'itinerary-item itinerary-item-selected'
                : 'itinerary-item'
            "
            :key="itinerary.generalizedCost + '' + itinerary.startTime"
            v-for="(itinerary, index) in itineraries"
            v-on:click="() => changeItinerary(index)"
          >
            <div class="itinerary-item-line"></div>
            <div
              v-for="leg in itinerary.legs"
              :key="leg.startTime"
              :style="{ position: 'relative' }"
            >
              <div
                :style="{
                  position: 'absolute',
                  backgroundColor: leg.transitLeg ? '#d11' : '#aaa',
                  left: `${
                    100 *
                    ((leg.startTime - earliestStart) /
                      (latestArrival - earliestStart))
                  }%`,
                  right: `${Math.round(
                    100 *
                      ((latestArrival - leg.endTime) /
                        (latestArrival - earliestStart))
                  )}%`,
                  height: '3em',
                  top: '-2em',
                  marginLeft: '0.2em',
                  marginRight: '0.2em',
                  borderRadius: '0.5em',
                }"
              >
                <q-icon
                  v-if="
                    leg.mode === 'BUS' &&
                    (leg.endTime - leg.startTime) /
                      (latestArrival - earliestStart) >
                      0.1
                  "
                  name="directions_bus"
                  color="black"
                  size="sm"
                  :style="{
                    top: '50%',
                    left: '50%',
                    transform: 'translate(-50%, -50%)',
                  }"
                ></q-icon>
                <q-icon
                  v-if="
                    leg.mode === 'WALK' &&
                    (leg.endTime - leg.startTime) /
                      (latestArrival - earliestStart) >
                      0.1
                  "
                  name="directions_walk"
                  color="black"
                  size="sm"
                  :style="{
                    top: '50%',
                    left: '50%',
                    transform: 'translate(-50%, -50%)',
                  }"
                ></q-icon>
                <q-icon
                  v-if="
                    (leg.mode === 'TRAIN' || leg.mode === 'TRAM') &&
                    (leg.endTime - leg.startTime) /
                      (latestArrival - earliestStart) >
                      0.1
                  "
                  name="directions_train"
                  color="black"
                  size="sm"
                  :style="{
                    top: '50%',
                    left: '50%',
                    transform: 'translate(-50%, -50%)',
                  }"
                ></q-icon>
              </div>
            </div>
          </li>
        </ul>
        <div></div>
      </q-card-section>
    </q-card>
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
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/utils/models';
import { defineComponent, Ref, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';
import { decodeOtpPath } from 'src/third_party/decodePath';
import { LngLat, LngLatBounds, Marker } from 'maplibre-gl';
import { useQuasar } from 'quasar';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

type LegGeometry = {
  points: string;
};

type ItineraryLeg = {
  startTime: number;
  endTime: number;
  mode: 'WALK' | 'BUS' | 'TRAIN' | 'TRAM';
  transitLeg: boolean;
  legGeometry: LegGeometry;
};

type Itinerary = {
  generalizedCost: number;
  startTime: number;
  endTime: number;
  legs: ItineraryLeg[];
};

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
  } {
    return {
      itineraries: [],
      itineraryIndex: 0,
      earliestStart: 0,
      latestArrival: 0,
    };
  },
  components: { SearchBox },
  methods: {
    poiDisplayName,
    changeItinerary(index: number) {
      this.$data.itineraryIndex = index;
      this.plotPath();
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
        const rawResponse = await fetch(
          `/otp/routers/default/plan?fromPlace=${fromPoi.value?.position.lat},${fromPoi.value?.position.long}&toPlace=${toPoi.value?.position.lat},${toPoi.value?.position.long}&numItineraries=8`
        );
        const response = await rawResponse.json();
        this.$data.itineraries = response.plan.itineraries.sort(
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          (a: any, b: any) => a.endTime - b.endTime
        );
        this.calculateStats();
        this.plotPath();
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
    plotPath() {
      const itinerary = this.$data.itineraries[this.$data.itineraryIndex];
      // FIXME: Here and above, don't hardcode a credible maximum number of legs, it's very silly, just store it.
      for (
        var credibleMaxIndex = 0;
        credibleMaxIndex < 100;
        credibleMaxIndex++
      ) {
        const layerName = `headway_polyline${credibleMaxIndex}`;
        if (map?.getLayer(layerName)) {
          map?.removeLayer(layerName);
        }
        if (map?.getSource(layerName)) {
          map?.removeSource(layerName);
        }
      }
      var bbox: [number, number, number, number] = [1000, 1000, -1000, -1000];
      for (var index in itinerary.legs) {
        const layerName = `headway_polyline${index}`;
        if (map?.getLayer(layerName)) {
          map?.removeLayer(layerName);
        }
        if (map?.getSource(layerName)) {
          map?.removeSource(layerName);
        }
        const points: number[][] = decodeOtpPath(
          itinerary.legs[index].legGeometry.points
        );
        for (var point in points) {
          if (points[point][0] < bbox[0]) {
            bbox[0] = points[point][0];
          }
          if (points[point][1] < bbox[1]) {
            bbox[1] = points[point][1];
          }
          if (points[point][0] > bbox[2]) {
            bbox[2] = points[point][0];
          }
          if (points[point][1] > bbox[3]) {
            bbox[3] = points[point][1];
          }
        }
        map?.addSource(layerName, {
          type: 'geojson',
          data: {
            type: 'Feature',
            properties: {},
            geometry: {
              type: 'LineString',
              coordinates: points,
            },
          },
        });
        map?.addLayer({
          id: layerName,
          type: 'line',
          source: layerName,
          layout: {
            'line-join': 'round',
            'line-cap': 'round',
          },
          paint: {
            'line-color': itinerary.legs[index].transitLeg
              ? '#E21919'
              : '#1976D2',
            'line-width': itinerary.legs[index].transitLeg ? 6 : 4,
            'line-dasharray': itinerary.legs[index].transitLeg ? [1] : [1, 2],
          },
        });
      }
      getBaseMap()?.fitBounds(
        new LngLatBounds(
          new LngLat(bbox[0], bbox[1]),
          new LngLat(bbox[2], bbox[3])
        )
      );
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
