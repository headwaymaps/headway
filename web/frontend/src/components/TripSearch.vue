<template>
  <div class="top-left-card">
    <q-card>
      <q-card-section :style="{ display: 'flex', alignItems: 'center' }">
        <search-box
          ref="searchBox"
          :hint="$t('search.from')"
          :style="{ flex: 1 }"
          :force-text="displayName(fromPlace)"
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
      </q-card-section>
      <q-card-section
        :style="{ paddingTop: 0, display: 'flex', alignItems: 'center' }"
      >
        <search-box
          ref="searchBox"
          :hint="$t('search.to')"
          :style="{ flex: 1 }"
          :force-text="displayName(toPlace)"
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
      </q-card-section>
      <q-card-section style="padding-top: 0">
        <travel-mode-bar
          :current-mode="currentMode"
          :to-place="toPlace"
          :from-place="fromPlace"
        />
      </q-card-section>
    </q-card>
  </div>
</template>
<script lang="ts">
import { TravelMode } from 'src/utils/models';
import { defineComponent, PropType } from 'vue';
import TravelModeBar from 'src/components/TravelModeBar.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import { LngLat } from 'maplibre-gl';

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
    displayName(place?: Place): string | undefined {
      if (!place) {
        return undefined;
      }

      return place.name;
    },
    didClickSwap() {
      this.didSwapPlaces(this.toPlace, this.fromPlace);
    },
    didClickFromGps() {
      const options = {
        enableHighAccuracy: true,
        maximumAge: 60000,
        timeout: 10000,
      };
      navigator.geolocation.getCurrentPosition(
        (position) => {
          let lngLat = new LngLat(
            position.coords.longitude,
            position.coords.latitude
          );
          let place = Place.bareLocation(lngLat);
          place.name = this.$t('my_location');
          this.didSelectFromPlace(place);
        },
        (error) => {
          console.error(error);
        },
        options
      );
    },
  },
});
</script>
