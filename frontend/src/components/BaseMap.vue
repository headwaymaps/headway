<template>
  <div id="map"></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import maplibregl from 'maplibre-gl';

var map = null;

export default defineComponent({
  name: 'BaseMap',
  mounted: function() {
    map = new maplibregl.Map({
        container: 'map', // container id
        style: '/styles/basic-preview/style.json', // style URL
        center: [0, 0], // starting position [lng, lat]
        zoom: 1 // starting zoom
    });
  }
});
</script>
