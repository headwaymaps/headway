<template>
  <q-layout view="lHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <search-box v-on:poi_selected="poiSelected"></search-box>
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
import BaseMap from '../components/BaseMap.vue';
import { Marker } from 'maplibre-gl';

var markers: Marker[] = [];

export default defineComponent({
  name: 'MainLayout',

  components: { SearchBox, BaseMap },

  methods: {
    poiSelected: function (poi?: POI) {
      markers.forEach((marker) => marker.remove());
      markers = [];
      if (poi?.position) {
        this.$refs.baseMap.flyTo(poi?.position, 16);
        console.log(poi.position);
        markers.push(
          this.$refs.baseMap.addMarker(
            new Marker({ color: '#111111' }).setLngLat([
              poi.position.long,
              poi.position.lat,
            ])
          )
        );
        console.log(markers);
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
