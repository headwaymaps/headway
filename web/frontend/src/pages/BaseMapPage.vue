<template>
  <q-card class="top-left-card">
    <q-card-section>
      <search-box ref="searchBox" v-model="poi"></search-box>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { getBaseMap, setBottomCardAllowance } from 'src/components/BaseMap.vue';
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
    getBaseMap()?.removeMarkersExcept([]);
    setTimeout(() => setBottomCardAllowance(0));
  },
  setup: function () {
    return {};
  },
});
</script>
