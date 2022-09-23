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
import { getBaseMap } from 'src/components/BaseMap.vue';
import { decanonicalizePoi, POI, poiDisplayName } from 'src/utils/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';

async function renderOnMap(poi: POI) {
  if (poi.bbox) {
    // prefer bounds when available so we don't "overzoom" on a large
    // entity like an entire city.
    getBaseMap()?.fitBounds(poi.bbox, { maxZoom: 16 });
  } else if (poi.position) {
    getBaseMap()?.flyTo([poi.position.long, poi.position.lat], 16);
  }

  if (poi.position) {
    getBaseMap()?.pushMarker(
      'active_marker',
      new Marker({ color: '#111111' }).setLngLat([
        poi.position.long,
        poi.position.lat,
      ])
    );
    getBaseMap()?.removeMarkersExcept(['active_marker']);
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
  watch: {},
  methods: {
    poiDisplayName,
  },
  mounted: async function () {
    const poi = await decanonicalizePoi(this.$props.osm_id);
    this.$data.poi = poi;

    if (poi) {
      await renderOnMap(poi);
      this.$emit('loadedPoi', poi);
    } else {
      console.warn(`unable to find POI with osm_id: ${this.$props.osm_id}`);
    }

    // watch *after* initial render
    this.$watch('poi', async (newValue) => {
      const gidComponent = encodeURIComponent(newValue.gid);
      this.$router.push(`/place/${gidComponent}`);

      await renderOnMap(newValue);
      this.$emit('loadedPoi', newValue);
    });
  },
  setup: function () {
    return {};
  },
});
</script>
