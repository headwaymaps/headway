<template>
  <div class="top-left-card" ref="searchCard">
    <q-card>
      <search-box
        ref="searchBox"
        hint="From"
        v-model="fromPoi"
        :force-text="fromPoi ? poiDisplayName(fromPoi) : undefined"
        v-on:update:model-value="rewriteUrl"
      ></search-box>
      <search-box
        ref="searchBox"
        hint="To"
        v-model="toPoi"
        :force-text="toPoi ? poiDisplayName(toPoi) : undefined"
        v-on:update:model-value="rewriteUrl"
      ></search-box>
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
  activeMarkers,
  map,
  setBottomCardAllowance,
} from 'src/components/BaseMap.vue';
import {
  canonicalizePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/components/models';
import { defineComponent, Ref, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';
import { decodeOtpPath } from 'src/third_party/decodePath';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'TransitPage',
  props: {
    mode: String,
    to: String,
    from: String,
  },
  data: function () {
    return {
      itineraries: [],
      itineraryIndex: 0,
      earliestStart: 0,
      latestArrival: 0,
      points: [],
    };
  },
  components: { SearchBox },
  methods: {
    poiDisplayName,
    changeItinerary(index: number) {
      this.$data.itineraryIndex = index;
      this.plotPath();
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
      for (var index = 0; index < 100; index++) {
        const layerName = `headway_polyline${index}`;
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
        setTimeout(() => {
          map?.fitBounds(bbox, {
            padding: {
              top: this.$refs.searchCard.offsetHeight + 20,
              bottom: 20,
            },
          });
        }, 100);
      }
    },
    resizeMap() {
      if (this.$refs.bottomCard && this.$refs.bottomCard) {
        setBottomCardAllowance(this.$refs.bottomCard.offsetHeight);
      } else {
        setBottomCardAllowance(0);
      }
    },
  },
  watch: {
    to(newValue) {
      setTimeout(async () => {
        toPoi.value = await decanonicalizePoi(newValue);
        await this.rewriteUrl();
        this.resizeMap();
      });
    },
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
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;
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
