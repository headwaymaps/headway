<template>
  <q-card class="top-left-card">
    <q-card-section>
      <search-box
        ref="searchBox"
        :force-text="poiDisplayName(poi)"
        v-model="poi"
      ></search-box>
    </q-card-section>
  </q-card>

  <place-card :poi="poi" v-on:close="$router.push('/')"></place-card>
</template>

<script lang="ts">
import { Marker } from 'maplibre-gl';
import { activeMarkers, getBaseMap, map } from 'src/components/BaseMap.vue';
import {
  encodePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/utils/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent } from 'vue';
import { Router } from 'vue-router';
import SearchBox from 'src/components/SearchBox.vue';

async function loadPlacePage(router: Router, canonicalName: string) {
  const poi = await decanonicalizePoi(canonicalName);

  if (poi?.position) {
    getBaseMap()?.flyTo([poi.position.long, poi.position.lat], 16);
    if (map) {
      const marker = new Marker({ color: '#111111' }).setLngLat([
        poi.position.long,
        poi.position.lat,
      ]);
      marker.addTo(map);
      activeMarkers.push(marker);
    }
    return poi;
  }
}

export default defineComponent({
  name: 'PlacePage',
  props: {
    osm_id: String,
  },
  emits: ['loadedPoi'],
  components: { PlaceCard, SearchBox },
  data: function () {
    return {
      poi: {},
    };
  },
  watch: {
    poi(newValue) {
      setTimeout(async () => {
        if (newValue) {
          await loadPlacePage(this.$router, encodePoi(newValue));
          this.$emit('loadedPoi', this.$data.poi);
        } else {
          this.$router.push('/');
        }
      });
    },
  },
  methods: {
    poiDisplayName,
    poiSelected: function (poi?: POI) {
      activeMarkers.forEach((marker) => marker.remove());
      activeMarkers.length = 0;
      if (poi?.gid) {
        const gidComponent = encodeURIComponent(poi?.gid);
        this.$router.push(`/place/${gidComponent}`);
      } else {
        this.$router.push('/');
      }
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      this.$data.poi = (await loadPlacePage(
        this.$router,
        this.$props.osm_id as string
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
      )) as any;
      this.$emit('loadedPoi', this.$data.poi);
    });
  },
  setup: function () {
    return {};
  },
});
</script>
