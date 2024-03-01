<template>
  <q-item-label>
    {{ $t('via_$place', { place: route.viaRoadsFormatted }) }}
  </q-item-label>
</template>

<script lang="ts">
import Route from 'src/models/Route';
import { defineComponent, PropType } from 'vue';
import Trip from 'src/models/Trip';

export default defineComponent({
  name: 'SingleModeListItem',
  data(): { route: Route } {
    // this cast is safe because we know that it's a non-transit trip
    const route = this.trip.nonTransitRoute() as Route;
    console.assert(route);
    return { route };
  },
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
