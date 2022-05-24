<template>
  <div id="map"></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import maplibregl from 'maplibre-gl';

var map = null;

async function loadMap() {
  const response = await fetch(`/bbox.txt`)
  if (response.status != 200) {
    // TODO surface error
    return
  }
  const bbox_strings = (await response.text()).split(' ')
  const bounds = [
    parseFloat(bbox_strings[0]),
    parseFloat(bbox_strings[1]),
    parseFloat(bbox_strings[2]),
    parseFloat(bbox_strings[3]),
  ];
  const extents = [bounds[2] - bounds[0], bounds[3] - bounds[1]]
  const maxExtent = Math.max(...extents) / 2
  const center = [(bounds[2] + bounds[0]) / 2, (bounds[3] + bounds[1]) / 2]
  const maxBounds = [center[0] - maxExtent, center[1] - maxExtent, center[0] + maxExtent, center[1] + maxExtent]
  map = new maplibregl.Map({
    container: 'map', // container id
    style: '/styles/basic-preview/style.json', // style URL
    center: [0, 0], // starting position [lng, lat]
    maxBounds: maxBounds,
    zoom: 1 // starting zoom
  });
}

export default defineComponent({
  name: 'BaseMap',
  mounted: async function() {
    await loadMap()
  }
});
</script>
