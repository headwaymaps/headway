<template>
  <q-card class="bottom-card" ref="bottomCard">
    <q-card-section class="bg-primary text-white">
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
        <div class="place-card-section">
          <q-chip
            icon="directions_bus"
            v-if="haveTransit"
            clickable
            v-on:click="
              $router.push(`/multimodal/${encodePoi($props.poi)}/_`)
            "
          >
            Bus there
          </q-chip>
          <q-chip
            icon="directions_car"
            clickable
            v-on:click="
              $router.push(`/directions/car/${encodePoi($props.poi)}/_`)
            "
          >
            Drive there
          </q-chip>
          <q-chip
            icon="directions_bike"
            clickable
            v-on:click="
              $router.push(
                `/directions/bicycle/${encodePoi($props.poi)}/_`
              )
            "
          >
            Bike there
          </q-chip>
          <q-chip
            icon="directions_walk"
            clickable
            v-on:click="
              $router.push(`/directions/walk/${encodePoi($props.poi)}/_`)
            "
          >
            Walk there
          </q-chip>
        </div>
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { setBottomCardAllowance } from './BaseMap.vue';
import { encodePoi } from './models';

export default defineComponent({
  name: 'PlaceCard',
  emits: ['close'],
  props: {
    poi: Object,
  },
  data() {
    return {
      haveTransit: false,
    }
  },
  methods: {
    encodePoi,
  },
  watch: {
    poi: function () {
      setTimeout(() => {
        setBottomCardAllowance(this.$refs.bottomCard.$el.offsetHeight);
      });
    },
  },
  mounted: async function () {
    setTimeout(() => {
      setBottomCardAllowance(this.$refs.bottomCard.$el.offsetHeight);
    });
    let response = await fetch("/capabilities.txt");
    if (response.status != 200) {
      // TODO surface error
      return false;
    }
    const capabilities = (await response.text()).split('\n');
    this.haveTransit = capabilities.find((val: string) => val === "OTP") !== undefined
  },
  components: {},
});
</script>
