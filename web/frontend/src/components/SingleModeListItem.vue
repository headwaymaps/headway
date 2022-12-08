<template>
  <q-item-label>
    {{ $t('via_$place', { place: item.viaRoadsFormatted }) }}
  </q-item-label>
</template>

<script lang="ts">
import Route from 'src/models/Route';
import { defineComponent, PropType } from 'vue';

export default defineComponent({
  name: 'SingleModeListItem',
  props: {
    item: {
      type: Object as PropType<Route>,
      required: true,
    },
    // SingleModalListItem actually doesn't use this, but MultiModalListItem needs it, so
    // we have to include it here to avoid an "unexpected property" warning.
    // This feels gross, but hopefully I can find a better way.
    active: Boolean,
    earliestStart: Number,
    latestArrival: Number,
  },
});
</script>
