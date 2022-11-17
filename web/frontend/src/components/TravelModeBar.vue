<template>
  <div class="travel-mode-bar">
    <q-btn
      icon="directions_bus"
      v-if="transitRoutingEnabled"
      unelevated
      rounded
      :ripple="false"
      size="sm"
      :to="`/multimodal/${poiToUrlArg(toPoi)}/${poiToUrlArg(fromPoi)}`"
      :color="currentMode === 'transit' ? 'primary' : undefined"
    >
      {{ $t('modes.transit') }}
    </q-btn>
    <q-btn
      icon="directions_car"
      unelevated
      rounded
      :ripple="false"
      size="sm"
      :to="`/directions/car/${poiToUrlArg(toPoi)}/${poiToUrlArg(fromPoi)}`"
      :color="currentMode === 'car' ? 'primary' : undefined"
    >
      {{ $t('modes.drive') }}
    </q-btn>
    <q-btn
      icon="directions_bike"
      unelevated
      rounded
      :ripple="false"
      size="sm"
      :to="`/directions/bicycle/${poiToUrlArg(toPoi)}/${poiToUrlArg(fromPoi)}`"
      :color="currentMode === 'bicycle' ? 'primary' : undefined"
    >
      {{ $t('modes.bike') }}
    </q-btn>
    <q-btn
      icon="directions_walk"
      unelevated
      rounded
      :ripple="false"
      size="sm"
      :to="`/directions/walk/${poiToUrlArg(toPoi)}/${poiToUrlArg(fromPoi)}`"
      :color="currentMode === 'walk' ? 'primary' : undefined"
    >
      {{ $t('modes.walk') }}
    </q-btn>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { POI, TravelMode, encodePoi } from 'src/utils/models';
import Config from 'src/utils/Config';

export default defineComponent({
  name: 'TravelModeBar',
  props: {
    currentMode: String as () => TravelMode,
    fromPoi: Object as () => POI,
    toPoi: Object as () => POI,
  },
  setup: function () {
    return { transitRoutingEnabled: Config.transitRoutingEnabled };
  },
  methods: {
    poiToUrlArg(poi?: POI): string {
      if (!poi) {
        return '_';
      } else {
        return encodePoi(poi);
      }
    },
  },
});
</script>
