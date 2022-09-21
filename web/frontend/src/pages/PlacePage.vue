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

async function loadPlacePage(poi: POI | undefined) {
  if (poi === undefined) {
    return;
  }

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
  watch: {
    poi(newValue) {
      setTimeout(async () => {
        if (newValue) {
          await loadPlacePage(this.$router, newValue);
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
      getBaseMap()?.removeMarkersExcept([]);
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
      const poi = await decanonicalizePoi(this.$props.osm_id);
      this.$data.poi = (await loadPlacePage(
        this.$router,
        poi
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
