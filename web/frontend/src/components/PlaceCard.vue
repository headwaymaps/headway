<template>
  <q-card class="bottom-card" ref="bottomCard">
    <q-card-section>
      <div class="place-card-conditionally-wrap">
        <q-btn
          flat
          round
          class="place-card-close-button"
          color="white"
          icon="close"
          v-on:click="$emit('close')"
        />
        <div class="place-card-section">
          <div class="text-subtitle1">
            {{ primaryName() }}
          </div>
          <div class="text" v-if="secondaryName()">
            {{ secondaryName() }}
          </div>
        </div>
      </div>
    </q-card-section>
    <q-card-section>
      <div class="place-card-section">
        <travel-mode-bar :to-place="place" />
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { QCard } from 'quasar';
import { i18n } from 'src/i18n/lang';
import Place from 'src/models/Place';
import { defineComponent } from 'vue';
import { setBottomCardAllowance } from './BaseMap.vue';
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
        return undefined;
      }
    },
  },
  watch: {
    poi: function () {
      setTimeout(() => {
        setBottomCardAllowance(
          (this.$refs.bottomCard as QCard).$el.offsetHeight
        );
      });
    },
  },
  mounted: async function () {
    setTimeout(() => {
      setBottomCardAllowance((this.$refs.bottomCard as QCard).$el.offsetHeight);
    });
  },
  components: { TravelModeBar },
});
</script>
