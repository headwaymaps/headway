<template>
  <q-card class="top-left-card">
    <q-card-section>
      <search-box
        ref="searchBox"
        :force-text="poiDisplayName(poi)"
        v-on:did-select-poi="searchBoxDidSelectPoi"
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
  components: { PlaceCard, SearchBox },
  data: function () {
    return {
      poi: {},
    };
  },
  watch: {
    poi: async function (newValue): Promise<void> {
      await renderOnMap(newValue);
    },
  },
  methods: {
    poiDisplayName,
    searchBoxDidSelectPoi(poi?: POI) {
      if (poi) {
        this.poi = poi;
      } else {
        this.$router.push('/');
      }
    },
  },
  beforeRouteUpdate: async function (to, from, next) {
    const newOsmId = to.params.osm_id;
    const poi = await decanonicalizePoi(newOsmId);
    if (poi) {
      this.poi = poi;
    } else {
      console.warn(`unable to find POI with osm_id: ${this.$props.osm_id}`);
    }

    next();
  },
  mounted: async function () {
    const poi = await decanonicalizePoi(this.$props.osm_id);
    if (poi) {
      this.$data.poi = poi;
    } else {
      console.warn(`unable to find POI with osm_id: ${this.$props.osm_id}`);
    }
  },
  setup: function () {
    return {};
  },
});
</script>
