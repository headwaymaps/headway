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
      <q-card-section class="bg-primary text-white">
        <div class="timeline" ref="timeline" v-on:scroll="scroll">
          <ol>
            <li :key="`${item.time}`" v-for="item in $data.steps">
              <div class="instruction">{{ item.instruction }}</div>
            </li>
          </ol>
        </div>
        <q-btn
          round
          color="black"
          class="center-left-floating"
          icon="chevron_left"
          v-on:click="scrollLeft"
        />
        <q-btn
          round
          color="black"
          class="center-right-floating"
          icon="chevron_right"
          v-on:click="scrollRight"
        />
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
      points: [],
    };
  },
  components: { SearchBox },
  methods: {
    poiDisplayName,
    scrollLeft() {
      const timeline = this.$refs.timeline as Element;
      const position =
        timeline.scrollLeft / (timeline.scrollWidth - timeline.clientWidth);
      const step = Math.min(
        Math.floor(position * this.$data.steps.length),
        this.$data.steps.length - 1
      );
      const newStep = Math.max(Math.ceil(step - 1), 0);
      const newPosition =
        (newStep / this.$data.steps.length) *
        (timeline.scrollWidth - timeline.clientWidth);
      timeline.scroll({ behavior: 'smooth', left: newPosition });
    },
    scrollRight() {
      const timeline = this.$refs.timeline as Element;
      const position =
        timeline.scrollLeft / (timeline.scrollWidth - timeline.clientWidth);
      const step = Math.min(
        Math.floor(position * this.$data.steps.length),
        this.$data.steps.length - 1
      );
      const newStep = Math.min(
        this.$data.steps.length - 1,
        Math.floor(step + 1)
      );
      const newPosition =
        (newStep / this.$data.steps.length) *
        (timeline.scrollWidth - timeline.clientWidth);
      timeline.scroll({ behavior: 'smooth', left: newPosition });
    },
    scroll() {
      const timeline = this.$refs.timeline as Element;
      const position =
        timeline.scrollLeft / (timeline.scrollWidth - timeline.clientWidth);
      const step = Math.min(
        Math.floor(position * this.$data.steps.length),
        this.$data.steps.length - 1
      );
      const fraction = position * this.$data.steps.length - step;
      const stepStartPositionIndex = this.$data.steps[step].begin_shape_index;
      const stepEndPositionIndex = this.$data.steps[step].end_shape_index;
      const stepPositionCount = stepEndPositionIndex - stepStartPositionIndex;
      const lerpFraction =
        fraction * stepPositionCount - Math.floor(fraction * stepPositionCount);
      const lerpStart =
        this.$data.points[
          stepStartPositionIndex + Math.floor(fraction * stepPositionCount)
        ];
      const lerpEnd =
        this.$data.points[
          stepStartPositionIndex + Math.ceil(fraction * stepPositionCount)
        ];
      const finalPos: [number, number] = [
        lerpEnd[0] * lerpFraction + lerpStart[0] * (1 - lerpFraction),
        lerpEnd[1] * lerpFraction + lerpStart[1] * (1 - lerpFraction),
      ];
      map?.easeTo({ center: finalPos, zoom: 16, duration: 0 });
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
          this.$data.points = points;
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
    };
  },
});
</script>
