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
    <div
      v-if="isEditable()"
      :style="{
        margin: '8px -16px',
        padding: '8px 16px',
        backgroundColor: showEditPanel ? '#eaeaea' : undefined,
      }"
    >
      <div style="text-align: right">
        <q-btn flat :ripple="false" @click="didToggleEdit">{{
          $t('edit_poi_button')
        }}</q-btn>
      </div>
      <div v-if="showEditPanel" style="margin-top: 8px">
        <p>{{ $t('edit_poi_about_osm') }}</p>
        <div style="text-align: center">
          <q-btn
            size="12px"
            flat
            :ripple="false"
            icon-right="launch"
            :href="osmEditUrl()"
            >{{ $t('edit_poi_on_osm_button') }}</q-btn
          >
        </div>
      </div>
    </div>
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
  data(): {
    showEditPanel: boolean;
  } {
    return { showEditPanel: false };
  },
  methods: {
    didToggleEdit(): void {
      this.showEditPanel = !this.showEditPanel;
    },
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
    isEditable(): boolean {
      return !!this.osmEditUrl();
    },
    osmEditUrl(): string | undefined {
      try {
        return this.place.id.editOSMVenueUrl()?.toString();
      } catch {
        return undefined;
      }
    },
  },
  components: { TravelModeBar, PlaceField },
});
</script>
