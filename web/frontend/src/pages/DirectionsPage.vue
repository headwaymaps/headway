<template>
  <div class="topLeftCard">
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
  <div class="bottomCard">
    <q-card>
      <q-card-section class="bg-primary text-white">
        <div class="text-subtitle1">
          {{
            from
              ? `Directions from ${poiDisplayName(fromPoi)} to ${poiDisplayName(
                  toPoi
                )}`
              : `Directions to ${poiDisplayName(toPoi)}`
          }}
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
  components: { SearchBox },
  methods: {
    poiDisplayName,
    rewriteUrl: async function () {
      const fromCanonical = fromPoi.value
        ? canonicalizePoi(fromPoi.value)
        : '_';
      const toCanonical = toPoi.value ? canonicalizePoi(toPoi.value) : '_';
      this.$router.replace(
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
          if (map?.getLayer('headway_polyline')) {
            map?.removeLayer('headway_polyline');
          }
          if (map?.getSource('headway_polyline')) {
            map?.removeSource('headway_polyline');
          }
          var points: number[][] = [];
          decode(legs[0].shape, 6).forEach((point) =>
            points.push([point[1], point[0]])
          );
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
              'line-color': '#888',
              'line-width': 8,
            },
          });
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
        });
      },
    },
    from: {
      immediate: true,
      deep: true,
      handler(newValue) {
        setTimeout(async () => {
          fromPoi.value = await decanonicalizePoi(newValue);
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
  },
  setup: function () {
    return { toPoi, fromPoi };
  },
});
</script>
