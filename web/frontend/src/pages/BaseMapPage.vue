<template>
  <q-card class="top-left-card">
    <q-card-section>
      <search-box ref="searchBox" v-model="poi"></search-box>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import {
  activeMarkers,
  addMapHandler,
  removeMapHandler,
  setBottomCardAllowance,
} from 'src/components/BaseMap.vue';
import { mapFeatureToPoi } from 'src/utils/models';
import SearchBox from 'src/components/SearchBox.vue';
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'DirectionsPage',
  components: { SearchBox },
  data: function () {
    return {
      poi: {},
      handler: 0,
    };
  },
  watch: {
    poi(newValue) {
      if (newValue?.gid) {
        const gidComponent = encodeURIComponent(newValue?.gid);
        this.$router.push(`/place/${gidComponent}`);
      } else {
        this.$router.push('/');
      }
    },
  },
  mounted: function () {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;
    this.handler = addMapHandler('longpress', (event) => {
      this.$router.push(`/pin/${event.lngLat.lng}/${event.lngLat.lat}/`);
    });
    this.handler = addMapHandler('poi_click', async (event) => {
      if (!event?.features) {
        console.warn('poi_click without features');
        return;
      }
      let poi = await mapFeatureToPoi(event?.features[0]);
      if (!poi?.gid) {
        console.error('Could not canonicalize map feature.');
        return;
      }
      const gidComponent = encodeURIComponent(poi?.gid);
      this.$router.push(`/place/${gidComponent}`);
    });
    setTimeout(() => setBottomCardAllowance(0));
  },
  unmounted: function () {
    removeMapHandler('longpress', this.handler);
  },
  setup: function () {
    return {};
  },
});
</script>
