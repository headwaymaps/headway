<template>
  <div class="top-card">
    <search-box
      :initial-place="place"
      @did-select-place="searchBoxDidSelectPlace"
      @did-submit-search="
        (searchText) =>
          $router.push(`/search/${encodeURIComponent(searchText)}`)
      "
    />
  </div>

  <div class="bottom-card">
    <place-card v-if="place" :place="place" />
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
  components: { PlaceCard, SearchBox },
  beforeRouteUpdate: async function (
    to: RouteLocation,
    from: RouteLocation,
    next: () => void,
  ) {
    const placeId = to.params.placeId as string;
    const place = await PlaceStorage.fetchFromSerializedId(placeId);
    if (place) {
      this.place = place;
    } else {
      console.warn(`unable to find Place with id: ${placeId}`);
    }

    next();
  },
  props: {
    placeId: {
      type: String,
      required: true,
    },
  },
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
  mounted: async function () {
    const placeId = this.$props.placeId as string;
    const place = await PlaceStorage.fetchFromSerializedId(placeId);
    if (place) {
      this.place = place;
    } else {
      console.warn(`unable to find Place with id: ${placeId}`);
    }
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
});
</script>
