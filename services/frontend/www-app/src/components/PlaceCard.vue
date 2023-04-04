<template>
  <q-card-section>
    <div class="text-subtitle1">
      {{ primaryName() }}
    </div>
  </q-card-section>
  <q-card-section>
    <travel-mode-bar :to-place="place" />
  </q-card-section>
  <q-card-section>
    <place-field
      v-if="secondaryName()"
      :copy-text="secondaryName()!"
      icon="location_on"
    >
      {{ secondaryName() }}
    </place-field>
    <place-field v-if="website()" :copy-text="website()!" icon="public">
      <a :href="website()">{{ website() }}</a>
    </place-field>
    <place-field v-if="phone()" :copy-text="phone()!" icon="phone">
      <a :href="'tel:' + phone()">{{ phone() }}</a>
    </place-field>
  </q-card-section>
</template>

<script lang="ts">
import { i18n } from 'src/i18n/lang';
import Place from 'src/models/Place';
import { formatLngLatAsLatLng } from 'src/utils/format';
import { defineComponent } from 'vue';
import TravelModeBar from './TravelModeBar.vue';
import PlaceField from './PlaceField.vue';

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
      // TODO: use pelias label?
      if (this.place.name && this.place.address) {
        return this.place.address;
      } else {
        return formatLngLatAsLatLng(this.place.point);
      }
    },
    website(): string | undefined {
      return this.place.website;
    },
    phone(): string | undefined {
      return this.place.phone;
    },
  },
  components: { TravelModeBar, PlaceField },
});
</script>
