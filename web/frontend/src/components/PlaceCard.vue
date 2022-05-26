<template>
  <q-card>
    <q-card-section class="bg-primary text-white">
      <div class="text-subtitle1">
        {{ $props.poi?.name ? $props.poi?.name : $props.poi?.address }}
      </div>
      <div class="text" v-if="$props.poi?.name && $props.poi?.address">
        {{ $props.poi?.address }}
      </div>
      <div>
        <q-btn
          flat
          round
          class="placeCardCloseButton"
          color="white"
          icon="close"
          v-on:click="$emit('close')"
        />
      </div>
      <div class="placeCardSection">
        <q-chip
          icon="directions_bus"
          clickable
          v-on:click="
            $router.push(`/directions/transit/${canonicalizePoi($props.poi)}`)
          "
        >
          Bus there
        </q-chip>
        <q-chip
          icon="directions_bike"
          clickable
          v-on:click="
            $router.push(`/directions/bicycle/${canonicalizePoi($props.poi)}`)
          "
        >
          Bike there
        </q-chip>
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { canonicalizePoi } from './models';

export default defineComponent({
  name: 'PlaceCard',
  emits: ['close'],
  props: {
    poi: Object,
  },
  methods: {
    canonicalizePoi,
  },
  components: {},
});
</script>
