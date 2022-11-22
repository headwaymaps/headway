<template>
  <div class="top-left-card">
    <q-card>
      <q-card-section :style="{ display: 'flex', alignItems: 'center' }">
        <search-box
          ref="searchBox"
          :hint="$t('search.from')"
          :style="{ flex: 1 }"
          :force-text="fromPoi ? poiDisplayName(fromPoi) : undefined"
          v-on:did-select-poi="didSelectFromPoi as any"
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
          :force-text="toPoi ? poiDisplayName(toPoi) : undefined"
          v-on:did-select-poi="didSelectToPoi as any"
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
          :to-poi="toPoi"
          :from-poi="fromPoi"
        />
      </q-card-section>
    </q-card>
  </div>
</template>
<script lang="ts">
import { POI, TravelMode, poiDisplayName } from 'src/utils/models';
import { defineComponent, PropType } from 'vue';
import TravelModeBar from 'src/components/TravelModeBar.vue';
import SearchBox from 'src/components/SearchBox.vue';

export default defineComponent({
  name: 'TripSearch',
  props: {
    fromPoi: { type: Object as () => POI },
    toPoi: { type: Object as () => POI },
    currentMode: { type: String as () => TravelMode, required: true },
    didSelectFromPoi: {
      type: Function as PropType<(newValue?: POI) => void>,
      required: true,
    },
    didSelectToPoi: {
      type: Function as PropType<(newValue?: POI) => void>,
      required: true,
    },
    didSwapPois: {
      type: Function as PropType<
        (newToValue?: POI, newFromValue?: POI) => void
      >,
      required: true,
    },
  },
  components: { SearchBox, TravelModeBar },
  methods: {
    poiDisplayName,
    didClickSwap() {
      this.didSwapPois(this.toPoi, this.fromPoi);
    },
    didClickFromGps() {
      const options = {
        enableHighAccuracy: true,
        maximumAge: 60000,
        timeout: 10000,
      };
      navigator.geolocation.getCurrentPosition(
        (position) => {
          this.didSelectFromPoi({
            name: this.$t('my_location'),
            position: {
              lat: position.coords.latitude,
              long: position.coords.longitude,
            },
          });
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
