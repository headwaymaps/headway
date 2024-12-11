<template>
  <q-item
    class="list-item"
    :clickable="!$props.active"
    active-class="list-item--selected"
    :active="$props.active"
    @click="$props.clickHandler"
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
