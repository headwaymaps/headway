<template>
  <div class="top-card">
    <search-box
      :initial-place="place"
      v-on:did-select-place="searchBoxDidSelectPlace"
      v-on:did-submit-search="
        (searchText) =>
          $router.push(`/search/${encodeURIComponent(searchText)}`)
      "
    />
  </div>

  <div class="bottom-card">
    <place-card :place="place" v-if="place" />
  </div>
</template>

<script lang="ts">
import { getBaseMap } from 'src/components/BaseMap.vue';
import { placeDisplayName } from 'src/i18n/utils';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent } from 'vue';
import { RouteLocation } from 'vue-router';
import SearchBox from 'src/components/SearchBox.vue';
import Place, { PlaceStorage } from 'src/models/Place';
import Markers from 'src/utils/Markers';

function renderOnMap(place: Place) {
  const map = getBaseMap();
  if (!map) {
    console.error('map was unexpectedly unset');
    return;
  }

  map.flyToPlace(place);

  map.pushMarker('active_marker', Markers.active().setLngLat(place.point));
  map.removeAllLayers();
  map.removeMarkersExcept(['active_marker']);
}

export default defineComponent({
  name: 'PlacePage',
  props: {
    placeId: {
      type: String,
      required: true,
    },
  },
  components: { PlaceCard, SearchBox },
  data: function (): { place?: Place } {
    return {
      place: undefined,
    };
  },
  watch: {
    place: async function (newValue): Promise<void> {
      renderOnMap(newValue);
    },
  },
  methods: {
    placeDisplayName,
    searchBoxDidSelectPlace(place?: Place) {
      if (place) {
        this.place = place;
      } else {
        this.$router.push('/');
      }
    },
  },
  beforeRouteUpdate: async function (
    to: RouteLocation,
    from: RouteLocation,
    next: () => void,
  ) {
    const placeId = to.params.placeId as string;
    let place = await PlaceStorage.fetchFromSerializedId(placeId);
    if (place) {
      this.place = place;
    } else {
      console.warn(`unable to find Place with id: ${placeId}`);
    }

    next();
  },
  mounted: async function () {
    const placeId = this.$props.placeId as string;
    let place = await PlaceStorage.fetchFromSerializedId(placeId);
    if (place) {
      this.place = place;
    } else {
      console.warn(`unable to find Place with id: ${placeId}`);
    }
  },
});
</script>
