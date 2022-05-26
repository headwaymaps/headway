<template>
  <div class="topLeftCard">
    <q-card>
      <search-box ref="searchBox" v-model="poi"></search-box>
    </q-card>
  </div>
</template>

<script lang="ts">
import { activeMarkers } from 'src/components/BaseMap.vue';
import { POI } from 'src/components/models';
import SearchBox from 'src/components/SearchBox.vue';
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'DirectionsPage',
  components: { SearchBox },
  unmounted: function () {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;
  },
  data: function () {
    return {
      poi: {},
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
  methods: {
    changeModel(poi: POI | undefined) {
      console.log('watch worked');
      console.log(poi);
    },
  },
  setup: function () {
    return {};
  },
});
</script>
