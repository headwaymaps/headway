<template>
  <div class="top-card">
    <div style="display: flex">
      <search-box
        :hint="$t('search.from')"
        :style="{ flex: 1 }"
        :force-place="fromPlace"
        v-on:did-select-place="didSelectFromPlace"
      />
      <q-btn
        size="small"
        :style="{ marginLeft: '0.5em', marginRight: 0 }"
        flat
        round
        color="primary"
        icon="gps_fixed"
        v-on:click="didClickFromGps"
      />
    </div>
    <div style="margin-top: 16px; display: flex">
      <search-box
        :hint="$t('search.to')"
        :style="{ flex: 1 }"
        :force-place="toPlace"
        v-on:did-select-place="didSelectToPlace"
      />
      <q-btn
        size="small"
        :style="{ marginLeft: '0.5em', marginRight: 0 }"
        flat
        round
        color="primary"
        icon="swap_vert"
        v-on:click="didClickSwap"
      />
    </div>
    <travel-mode-bar
      style="margin-top: 16px"
      :current-mode="currentMode"
      :to-place="toPlace"
      :from-place="fromPlace"
    />
  </div>
</template>
<script lang="ts">
import { TravelMode } from 'src/utils/models';
import { defineComponent, PropType } from 'vue';
import TravelModeBar from 'src/components/TravelModeBar.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import { LngLat } from 'maplibre-gl';
import { placeDisplayName } from 'src/i18n/utils';
import env from 'src/utils/env';

export default defineComponent({
  name: 'TripSearch',
  props: {
    fromPlace: {
      type: Place,
      required: false,
    },
    toPlace: {
      type: Place,
      required: false,
    },
    currentMode: { type: String as () => TravelMode, required: true },
    didSelectFromPlace: {
      type: Function as PropType<(newValue?: Place) => void>,
      required: true,
    },
    didSelectToPlace: {
      type: Function as PropType<(newValue?: Place) => void>,
      required: true,
    },
    didSwapPlaces: {
      type: Function as PropType<
        (newToValue?: Place, newFromValue?: Place) => void
      >,
      required: true,
    },
  },
  components: { SearchBox, TravelModeBar },
  methods: {
    didClickSwap() {
      this.didSwapPlaces(this.toPlace, this.fromPlace);
    },
    didClickFromGps() {
      const options = {
        enableHighAccuracy: true,
        maximumAge: 60000,
        timeout: 10000,
      };

      env.geolocation.getCurrentPosition(
        (position: GeolocationPosition) => {
          let lngLat = new LngLat(
            position.coords.longitude,
            position.coords.latitude
          );
          let place = Place.bareLocation(lngLat);
          place.name = this.$t('my_location');
          this.didSelectFromPlace(place);
        },
        console.error,
        options
      );
    },
    placeDisplayName,
  },
});
</script>
