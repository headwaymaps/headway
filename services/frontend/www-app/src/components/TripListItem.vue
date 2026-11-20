<template>
  <q-item
    class="list-item trip-list-item"
    :clickable="!$props.active"
    active-class="list-item--selected"
    :active="$props.active"
    @click="$props.clickHandler"
  >
    <q-item-section class="col-9" top>
      <slot />
    </q-item-section>
    <q-item-section class="col-3" top style="text-align: right">
      <q-item-label class="hw-figure">
        {{ durationFormatted }}
      </q-item-label>
      <q-item-label class="hw-secondary">
        {{ distanceFormatted }}
      </q-item-label>
    </q-item-section>
  </q-item>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { QItemProps } from 'quasar';

export default defineComponent({
  name: 'TripListItem',
  props: {
    active: Boolean,
    clickHandler: {
      type: Function as PropType<QItemProps['onClick']>,
      default: () => {},
    },
    durationFormatted: {
      type: String,
      required: true,
    },
    distanceFormatted: {
      type: String,
      required: true,
    },
  },
});
</script>

<style lang="scss">
// Outlined rather than shaded on hover: a gray fill would swallow the gray walking pills.
@media (any-hover: hover) {
  .trip-list-item.q-hoverable:hover {
    box-shadow: inset 0 0 0 2px rgba(0, 0, 0, 0.2);
  }
  .trip-list-item.q-hoverable:hover > .q-focus-helper {
    opacity: 0;
  }
}
</style>
