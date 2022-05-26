<template>
  <q-layout view="lHh Lpr lFf">
    <q-header elevated>
      <q-toolbar>
        <search-box
          ref="searchBox"
          v-on:poi_selected="poiSelected"
          v-on:poi_hovered="poiHovered"
        ></search-box>
      </q-toolbar>
    </q-header>
    <div class="mainContainer">
      <router-view v-on:loadedPoi="propagatePoiSelection"></router-view>
      <base-map v-on:on-map-long-press="dropPin"></base-map>
    </div>
  </q-layout>
</template>

<script lang="ts">
import { POI } from 'src/components/models';
import { defineComponent, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';
import BaseMap, { activeMarkers, map } from 'src/components/BaseMap.vue';
import { MapMouseEvent, Marker } from 'maplibre-gl';

var hoverMarkers: Marker[] = [];

export default defineComponent({
  name: 'MainLayout',

  components: { SearchBox, BaseMap },
  props: {
    osm_id: String,
  },
  methods: {
    dropPin: function (event: MapMouseEvent) {
      this.$router.push(`/pin/${event.lngLat.lng}/${event.lngLat.lat}/`);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (this.$refs.searchBox as any).setPoi({});
    },
    propagatePoiSelection: function (poi?: POI) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (this.$refs.searchBox as any).setPoi(poi);
    },
    poiSelected: function (poi?: POI) {
      activeMarkers.forEach((marker) => marker.remove());
      activeMarkers.length = 0;
      hoverMarkers.forEach((marker) => marker.remove());
      hoverMarkers = [];
      if (poi?.id) {
        this.$router.push(`/place/${poi?.type}${poi?.id}`);
      } else {
        this.$router.push('/');
      }
      setTimeout(() => {
        hoverMarkers.forEach((marker) => marker.remove());
        hoverMarkers = [];
      }, 1000);
    },
    poiHovered: function (poi?: POI) {
      hoverMarkers.forEach((marker) => marker.remove());
      hoverMarkers = [];
      if (poi?.position && map && !this.$props.osm_id) {
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
