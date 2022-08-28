<template>
  <div class="top-left-card">
    <q-card>
      <search-box
        ref="searchBox"
        v-on:poi_selected="poiSelected"
        v-on:poi_hovered="poiHovered"
      ></search-box>
    </q-card>
  </div>

  <div class="bottom-card">
    <place-card :poi="poi" v-on:close="$router.push('/')"></place-card>
  </div>
</template>

<script lang="ts">
import { Marker } from 'maplibre-gl';
import { activeMarkers, getBaseMap, map } from 'src/components/BaseMap.vue';
import { LongLat, POI } from 'src/components/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent, Ref, ref } from 'vue';
import { Router } from 'vue-router';
import SearchBox from 'src/components/SearchBox.vue';

var poi: Ref<POI | undefined> = ref(undefined);

async function loadDroppedPinPage(
  router: Router,
  position: LongLat,
) {
  if (!map) {
    setTimeout(() => loadDroppedPinPage(router, position), 100);
    return;
  }
  poi.value = {
    name: 'Dropped Pin', // i18n
    address: undefined,
    position: position,
    id: undefined,
    type: undefined,
  };

  getBaseMap()?.flyTo(
    [position.long, position.lat],
    16,
  );
  if (map) {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;

    const marker = new Marker({ color: '#111111' }).setLngLat([
      position.long,
      position.lat,
    ]);
    marker.addTo(map);
    activeMarkers.push(marker);
  }
}

var hoverMarkers: Marker[] = [];

export default defineComponent({
  name: 'DroppedPinPage',
  props: {
    long: String,
    lat: String,
  },
  components: { PlaceCard, SearchBox },
  watch: {
    lat: {
      immediate: true,
      deep: true,
      handler() {
        setTimeout(async () => {
          const position: LongLat = {
            long: parseFloat(this.$props.long as string),
            lat: parseFloat(this.$props.lat as string),
          };
          await loadDroppedPinPage(this.$router, position);
        });
      },
    },
    long: {
      immediate: true,
      deep: true,
      handler() {
        setTimeout(async () => {
          const position: LongLat = {
            long: parseFloat(this.$props.long as string),
            lat: parseFloat(this.$props.lat as string),
          };
          await loadDroppedPinPage(this.$router, position);
        });
      },
    },
  },
  methods: {
    poiSelected: function (poi?: POI) {
      activeMarkers.forEach((marker) => marker.remove());
      activeMarkers.length = 0;
      hoverMarkers.forEach((marker) => marker.remove());
      hoverMarkers = [];
      if (poi?.gid) {
        const gidComponent = encodeURIComponent(poi?.gid);
        this.$router.push(`/place/${gidComponent}`);
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

  mounted: async function () {
    setTimeout(async () => {
      const position: LongLat = {
        long: parseFloat(this.$props.long as string),
        lat: parseFloat(this.$props.lat as string),
      };
      await loadDroppedPinPage(this.$router, position, true);
    });
  },
  setup: function () {
    return { poi };
  },
});
</script>
