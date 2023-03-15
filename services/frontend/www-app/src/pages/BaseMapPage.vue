<template>
  <div class="top-card">
    <search-box
      :tabindex="1"
      v-on:did-select-place="searchBoxDidSelectPlace"
      v-on:did-submit-search="
        (searchText) =>
          $router.push(`/search/${encodeURIComponent(searchText)}`)
      "
    />
  </div>
</template>

<style lang="scss">
// override some styles from the default layout.
.front-page {
  .top-card {
    @media screen and (max-width: 800px) {
      width: 100%;
      padding: 16px;
      border-bottom: solid #ccc 1px;
    }

    @media screen and (min-width: 800px) {
      z-index: 1;
      position: absolute;
      left: 0;
      width: calc(max(33%, 370px) - 32px);
      margin: 16px;
      border: none;
      padding: 0;
      // q-card style
      box-shadow: 0 1px 5px rgba(0, 0, 0, 0.2), 0 2px 2px rgba(0, 0, 0, 0.14),
        0 3px 1px -2px rgba(0, 0, 0, 0.12);
      border-radius: 4px;
    }
  }

  @media screen and (min-width: 800px) {
    #map {
      width: 100%;
      flex: 1;
    }
    #map:before {
      // hide left inner shadow from default layout.
      content: none;
    }
  }
}
</style>

<script lang="ts">
import { getBaseMap } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'BaseMapPage',
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
  },
});
</script>
