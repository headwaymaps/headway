<template>
  <q-item
    class="q-my-sm"
    clickable
    v-ripple
    active-class="bg-blue-1"
    :active="$props.active"
    v-on:click="$props.clickHandler"
  >
    <q-item-section class="col-10" top>
      <slot />
    </q-item-section>
    <q-item-section class="col-2" top style="text-align: right">
      <q-item-label>
        {{ durationFormatted }}
      </q-item-label>
      <q-item-label>
        <span class="text-weight-light">{{ distanceFormatted }}</span>
      </q-item-label>
    </q-item-section>
  </q-item>
  <q-separator spaced />
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'RouteListItem',
  props: {
    active: Boolean,
    clickHandler: Function,
    durationFormatted: String,
    distanceFormatted: String,
  },
});
</script>
