<template>
  <q-card class="top-left-card">
    <q-card-section>
      <search-box
        ref="searchBox"
        v-on:did-select-place="searchBoxDidSelectPlace"
      ></search-box>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { getBaseMap, setBottomCardAllowance } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'DirectionsPage',
  components: { SearchBox },
  methods: {
    searchBoxDidSelectPlace(place?: Place) {
      if (place) {
        this.$router.push(`/place/${place.urlEncodedId()}`);
      }
    },
  },
  mounted: function () {
    getBaseMap()?.removeAllMarkers();
    setTimeout(() => setBottomCardAllowance(0));
  },
});
</script>
