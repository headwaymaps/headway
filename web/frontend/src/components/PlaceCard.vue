<template>
  <q-card>
    <q-card-section class="bg-primary text-white">
      <div class="text-subtitle1">
        {{ $props.poi?.name ? $props.poi?.name : $props.poi?.address }}
      </div>
      <div class="text" v-if="$props.poi?.name && $props.poi?.address">
        {{ $props.poi?.address }}
      </div>
    </q-card-section>
  </q-card>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'PlaceCard',
  props: {
    poi: Object,
  },
  components: {},
});
</script>
