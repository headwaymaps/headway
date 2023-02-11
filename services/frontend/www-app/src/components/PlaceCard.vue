<template>
  <q-card-section>
    <div class="text-subtitle1">
      {{ primaryName() }}
    </div>
    <div class="text" v-if="secondaryName()">
      {{ secondaryName() }}
    </div>
  </q-card-section>
  <q-card-section>
    <travel-mode-bar :to-place="place" />
  </q-card-section>
</template>

<script lang="ts">
import { i18n } from 'src/i18n/lang';
import Place from 'src/models/Place';
import { formatLngLatAsLatLng } from 'src/utils/format';
import { defineComponent } from 'vue';
import TravelModeBar from './TravelModeBar.vue';

export default defineComponent({
  name: 'PlaceCard',
  emits: ['close'],
  props: {
    place: {
      type: Place,
      required: true,
    },
  },
  methods: {
    primaryName(): string {
      if (this.place.name) {
        return this.place.name;
      }
      if (this.place.address) {
        return this.place.address;
      }
      return i18n.global.t('dropped_pin');
    },
    secondaryName(): string | undefined {
      if (this.place.name && this.place.address) {
        return this.place.address;
      } else {
        return formatLngLatAsLatLng(this.place.point);
      }
    },
  },
  components: { TravelModeBar },
});
</script>
