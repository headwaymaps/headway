<template>
  <q-item-label>
    {{ $t('via_$place', { place: item.viaRoadsFormatted }) }}
  </q-item-label>
  <q-item-label>
    <q-btn
      style="margin-left: -6px"
      padding="6px"
      flat
      icon="directions"
      :label="$t('route_picker_show_route_details_btn')"
      size="sm"
      v-on:click="showRouteSteps(item)"
    />
  </q-item-label>
</template>

<script lang="ts">
import Route from 'src/models/Route';
import { defineComponent, PropType } from 'vue';

export default defineComponent({
  name: 'SingleModeListItem',
  props: {
    showRouteSteps: {
      type: Function as PropType<(route: Route) => void>,
      required: true,
    },
    item: {
      type: Object as PropType<Route>,
      required: true,
    },
  },
});
</script>
