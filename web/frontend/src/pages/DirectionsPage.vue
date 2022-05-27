<template>
  <div class="top-left-card">
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
  <div class="bottom-card" v-if="fromPoi && toPoi">
    <q-card>
      <q-btn
        flat
        round
        class="place-card-close-button"
        color="white"
        icon="close"
        v-on:click="$router.push('/')"
      />
      <q-card-section class="bg-primary text-white">
        <div class="text-subtitle1">
          {{ `${poiDisplayName(fromPoi)} to ${poiDisplayName(toPoi)}` }}
        </div>
      </q-card-section>
      <q-card-section class="bg-primary text-white">
        <div class="timeline">
          <ol>
            <li :key="`${item.time}`" v-for="item in $data.steps">
              <div class="instruction">{{ item.instruction }}</div>
            </li>
          </ol>
        </div>
      </q-card-section>
    </q-card>
  </div>
</template>

<script lang="ts">
import { activeMarkers, map } from 'src/components/BaseMap.vue';
import {
  canonicalizePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/components/models';
import { defineComponent, Ref, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';
import { decode } from 'src/components/utils';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'DirectionsPage',
  props: {
    mode: String,
    to: String,
    from: String,
  },
  data: function () {
    return {
      steps: [],
    };
  },
  components: { SearchBox },
  methods: {
    poiDisplayName,
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
        `/directions/${this.mode}/${toCanonical}/${fromCanonical}`
      );
      if (fromPoi.value?.position && toPoi.value?.position) {
        const requestObject = {
          locations: [
            {
              lon: fromPoi.value?.position.long,
              lat: fromPoi.value.position.lat,
            },
            { lon: toPoi.value?.position.long, lat: toPoi.value.position.lat },
          ],
          costing: 'bicycle',
          directions_options: {
            language: 'en-US', // FIXME: don't hardcode languages!
          },
        };
        const response = await fetch(
          `/valhalla/route?json=${JSON.stringify(requestObject)}`
        );
        const route = await response.json();
        const legs = route?.trip?.legs;
        if (legs && map) {
          this.$data.steps = legs[0].maneuvers;
          if (map?.getLayer('headway_polyline')) {
            map?.removeLayer('headway_polyline');
          }
          if (map?.getSource('headway_polyline')) {
            map?.removeSource('headway_polyline');
          }
          // min/max
          const bbox: number[] = [1000, 1000, -1000, -1000];
          var points: number[][] = [];
          decode(legs[0].shape, 6).forEach((point) => {
            points.push([point[1], point[0]]);
            if (point[1] < bbox[0]) {
              bbox[0] = point[1];
            }
            if (point[0] < bbox[1]) {
              bbox[1] = point[0];
            }
            if (point[1] > bbox[2]) {
              bbox[2] = point[1];
            }
            if (point[0] > bbox[3]) {
              bbox[3] = point[0];
            }
          });
          const center = [(bbox[0] + bbox[2]) / 2, (bbox[1] + bbox[3]) / 2];
          const extents = [bbox[2] - bbox[0], bbox[3] - bbox[1]];
          const zoomBounds: [number, number, number, number] = [
            center[0] - extents[0],
            center[1] - extents[1],
            center[0] + extents[0],
            center[1] + extents[1],
          ];
          map?.addSource('headway_polyline', {
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
            id: 'headway_polyline',
            type: 'line',
            source: 'headway_polyline',
            layout: {
              'line-join': 'round',
              'line-cap': 'round',
            },
            paint: {
              'line-color': '#1976D2',
              'line-width': 6,
            },
          });
          map?.fitBounds(zoomBounds);
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
  },
  watch: {
    to: {
      immediate: true,
      deep: true,
      handler(newValue) {
        setTimeout(async () => {
          toPoi.value = await decanonicalizePoi(newValue);
          this.rewriteUrl();
        });
      },
    },
    from: {
      immediate: true,
      deep: true,
      handler(newValue) {
        setTimeout(async () => {
          fromPoi.value = await decanonicalizePoi(newValue);
          this.rewriteUrl();
        });
      },
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      toPoi.value = await decanonicalizePoi(this.$props.to as string);
      fromPoi.value = await decanonicalizePoi(this.$props.from as string);
      await this.rewriteUrl();
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
      thumbStyle: {
        right: '4px',
        borderRadius: '5px',
        backgroundColor: '#111',
        width: '9px',
        opacity: '0.75',
      },

      barStyle: {
        right: '2px',
        borderRadius: '9px',
        backgroundColor: '#111',
        width: '5px',
        opacity: '0.2',
      },
      stepsListStyle: {
        whiteSpace: 'nowrap',
        overflow: 'auto',
        display: 'table',
      },
      stepsItemStyle: {
        display: 'inline-block',
        height: '100%',
      },
    };
  },
});
</script>
