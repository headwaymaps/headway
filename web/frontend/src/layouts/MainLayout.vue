<template>
  <q-layout view="lHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <search-box
          v-on:poi_selected="poiSelected"
          v-on:poi_hovered="poiHovered"
        ></search-box>
      </q-toolbar>
    </q-header>
    <q-page-container>
      <base-map ref="baseMap"></base-map>
    </q-page-container>
  </q-layout>
</template>

<script lang="ts">
import { POI } from 'src/components/models';
import { defineComponent, ref } from 'vue';
import SearchBox from '../components/SearchBox.vue';
import BaseMap, { map } from '../components/BaseMap.vue';
import { Marker } from 'maplibre-gl';

var markers: Marker[] = [];
var hoverMarkers: Marker[] = [];

export default defineComponent({
  name: 'MainLayout',

  components: { SearchBox, BaseMap },

  methods: {
    poiSelected: function (poi?: POI) {
      hoverMarkers.forEach((marker) => marker.remove());
      hoverMarkers = [];
      markers.forEach((marker) => marker.remove());
      markers = [];
      if (poi?.position && map) {
        map.flyTo({
          center: [poi.position.long, poi.position.lat],
          zoom: 16,
        });
        const marker = new Marker({ color: '#111111' }).setLngLat([
          poi.position.long,
          poi.position.lat,
        ]);
        marker.addTo(map);
        markers.push(marker);
      }
    },
    poiHovered: function (poi?: POI) {
      hoverMarkers.forEach((marker) => marker.remove());
      hoverMarkers = [];
      if (poi?.position && map) {
        const marker = new Marker({ color: '#11111155' }).setLngLat([
          poi.position.long,
          poi.position.lat,
        ]);
        marker.addTo(map);
        hoverMarkers.push(marker);
      }
    },
  },

  setup() {
    const leftDrawerOpen = ref(false);

    return {
      leftDrawerOpen,
      toggleLeftDrawer() {
        leftDrawerOpen.value = !leftDrawerOpen.value;
      },
    };
  },
});
</script>
