<template>
  <q-item
    class="route-list-item"
    clickable
    v-ripple
    active-class="route-list-item--selected"
    :active="$props.active"
    v-on:click="$props.clickHandler"
  >
    <q-item-section class="col-9" top>
      <slot />
    </q-item-section>
    <q-item-section class="col-3" top style="text-align: right">
      <q-item-label>
        {{ durationFormatted }}
      </q-item-label>
      <q-item-label>
        <span class="text-weight-light">{{ distanceFormatted }}</span>
      </q-item-label>
    </q-item-section>
  </q-item>
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
