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
            {{ $props.poi?.name ? $props.poi?.name : $props.poi?.address }}
          </div>
          <div class="text" v-if="$props.poi?.name && $props.poi?.address">
            {{ $props.poi?.address }}
          </div>
        </div>
      </div>
    </q-card-section>
    <q-card-section>
      <div class="place-card-section">
        <travel-mode-bar :to-poi="$props.poi" />
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { QCard } from 'quasar';
import { encodePoi } from 'src/utils/models';
import { defineComponent } from 'vue';
import { setBottomCardAllowance } from './BaseMap.vue';
import TravelModeBar from './TravelModeBar.vue';

export default defineComponent({
  name: 'PlaceCard',
  emits: ['close'],
  props: {
    poi: Object,
  },
  methods: {
    encodePoi,
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
