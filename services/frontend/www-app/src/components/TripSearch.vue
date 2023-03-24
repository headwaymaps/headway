<template>
  <div>
    <div style="display: flex">
      <search-box
        :hint="$t('search.from')"
        :style="{ flex: 1 }"
        :initial-place="fromPlace"
        :tabindex="1"
        v-on:did-select-place="didSelectFromPlace"
      />
      <q-btn
        size="12px"
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
        :initial-place="toPlace"
        :tabindex="2"
        v-on:did-select-place="didSelectToPlace"
      />
      <q-btn
        size="12px"
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
    <div :hidden="!isTransit()" style="margin-top: 8px; margin-bottom: -8px">
      <div v-if="searchTime || searchDate">
        <q-btn
          flat
          :label="$t('trip_search_depart_at')"
          size="sm"
          @click="didClickDepartAt"
        />
        <input
          type="time"
          :value="initialSearchTime"
          @change="(event) => searchTime = (event.target as HTMLInputElement).value"
        />
        <input
          type="date"
          :value="initialSearchDate"
          @change="(event) => searchDate = (event.target as HTMLInputElement).value"
        />
      </div>
      <q-btn
        v-else
        flat
        :label="$t('trip_search_depart_now')"
        size="sm"
        @click="didClickDepartNow"
      />
    </div>
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
    initialSearchTime: {
      type: String,
    },
    initialSearchDate: {
      type: String,
    },
    timeDidChange: {
      type: Function as PropType<(newValue: string) => void>,
      required: true,
    },
    dateDidChange: {
      type: Function as PropType<(newValue: string) => void>,
      required: true,
    },
  },
  data(): { searchTime?: string; searchDate?: string } {
    return {
      searchTime: this.initialSearchTime,
      searchDate: this.initialSearchDate,
    };
  },
  watch: {
    searchTime: function (newValue: string) {
      this.timeDidChange(newValue);
    },
    searchDate: function (newValue: string) {
      this.dateDidChange(newValue);
    },
  },
  components: { SearchBox, TravelModeBar },
  methods: {
    isTransit(): boolean {
      return this.currentMode == TravelMode.Transit;
    },
    didClickDepartAt() {
      this.searchTime = undefined;
      this.searchDate = undefined;
    },
    didClickDepartNow() {
      // BRITTLE: search date needs to be set first
      // beacuse OTP will error if a time is set without a date.
      this.searchDate = dateToInput(new Date());
      this.searchTime = timeToInput(new Date());
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

function dateToInput(date: Date) {
  return (
    date.getFullYear() +
    '-' +
    ('0' + (date.getMonth() + 1)).slice(-2) +
    '-' +
    ('0' + date.getDate()).slice(-2)
  );
}

function timeToInput(date: Date) {
  return (
    ('0' + date.getHours()).slice(-2) +
    ':' +
    ('0' + date.getMinutes()).slice(-2)
  );
}
</script>
