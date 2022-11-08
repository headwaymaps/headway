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
        <div class="place-card-section">
          <q-chip
            icon="directions_bus"
            v-if="haveTransit"
            clickable
            v-on:click="$router.push(`/multimodal/${encodePoi($props.poi)}/_`)"
          >
            {{ $t('modes.transit') }}
          </q-chip>
          <q-chip
            icon="directions_car"
            clickable
            v-on:click="
              $router.push(`/directions/car/${encodePoi($props.poi)}/_`)
            "
          >
            {{ $t('modes.drive') }}
          </q-chip>
          <q-chip
            icon="directions_bike"
            clickable
            v-on:click="
              $router.push(`/directions/bicycle/${encodePoi($props.poi)}/_`)
            "
          >
            {{ $t('modes.bike') }}
          </q-chip>
          <q-chip
            icon="directions_walk"
            clickable
            v-on:click="
              $router.push(`/directions/walk/${encodePoi($props.poi)}/_`)
            "
          >
            {{ $t('modes.walk') }}
          </q-chip>
        </div>
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { QCard } from 'quasar';
import { encodePoi } from 'src/utils/models';
import Config from 'src/utils/Config';
import { defineComponent } from 'vue';
import { setBottomCardAllowance } from './BaseMap.vue';

export default defineComponent({
  name: 'PlaceCard',
  emits: ['close'],
  props: {
    poi: Object,
  },
  data() {
    return {
      haveTransit: false,
    };
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
    this.haveTransit = Config.transitRoutingEnabled;
  },
  components: {},
});
</script>
