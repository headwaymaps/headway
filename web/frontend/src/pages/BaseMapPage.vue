<template>
  <div class="topLeftCard">
    <q-card>
      <search-box ref="searchBox" v-model="poi"></search-box>
    </q-card>
  </div>
</template>

<script lang="ts">
import {
  activeMarkers,
  addMapHandler,
  removeMapHandler,
} from 'src/components/BaseMap.vue';
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
      if (newValue?.id) {
        this.$router.push(`/place/${newValue?.type}${newValue?.id}`);
      } else {
        this.$router.push('/');
      }
    },
  },
  mounted: function () {
    this.handler = addMapHandler('longpress', (event) => {
      this.$router.push(`/pin/${event.lngLat.lng}/${event.lngLat.lat}/`);
    });
  },
  unmounted: function () {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;
    removeMapHandler('longpress', this.handler);
  },
  setup: function () {
    return {};
  },
});
</script>
