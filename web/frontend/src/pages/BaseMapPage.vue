<template>
  <q-card class="top-left-card">
    <q-card-section>
      <search-box
        ref="searchBox"
        v-on:did-select-poi="searchBoxDidSelectPoi"
      ></search-box>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { getBaseMap, setBottomCardAllowance } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import { POI } from 'src/utils/models';
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'DirectionsPage',
  components: { SearchBox },
  methods: {
    searchBoxDidSelectPoi(poi?: POI) {
      if (poi) {
        if (poi.gid) {
          const gidComponent = encodeURIComponent(poi.gid);
          this.$router.push(`/place/${gidComponent}`);
        } else {
          console.warn('search box POI had no GID', poi);
        }
      }
    },
  },
  data: function () {
    return {
      poi: {},
      handler: 0,
    };
  },
  mounted: function () {
    getBaseMap()?.removeAllMarkers();
    setTimeout(() => setBottomCardAllowance(0));
  },
  setup: function () {
    return {};
  },
});
</script>
