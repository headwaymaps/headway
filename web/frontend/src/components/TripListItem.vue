<template>
  <q-item
    class="trip-list-item"
    :clickable="!$props.active"
    active-class="trip-list-item--selected"
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

<style lang="scss">
.trip-list-item {
  border-bottom: solid $separator 1px;
  padding: 16px;
}

.trip-list-item--selected {
  border-left: solid $accent 8px;
  padding-left: 8px;
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'TripListItem',
  props: {
    active: Boolean,
    clickHandler: Function,
    durationFormatted: String,
    distanceFormatted: String,
  },
});
</script>
