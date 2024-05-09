<template>
  <q-item-label v-if="trip.viaRoadsFormatted">
    {{ $t('via_$place', { place: trip.viaRoadsFormatted }) }}
  </q-item-label>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import Trip from 'src/models/Trip';

export default defineComponent({
  name: 'SingleModeListItem',
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    // SingleModalListItem actually doesn't use this, but MultiModalListItem needs it, so
    // we have to include it here to avoid an "unexpected property" warning.
    // This feels gross, but hopefully I can find a better way.
    active: Boolean,
  },
});
</script>
