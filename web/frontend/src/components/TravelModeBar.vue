<template>
  <div class="travel-mode-bar">
    <q-btn
      icon="directions_bus"
      v-if="transitRoutingEnabled"
      unelevated
      rounded
      :ripple="false"
      size="sm"
      :to="linkPath('transit')"
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
      :to="linkPath('car')"
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
      :to="linkPath('bicycle')"
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
      :to="linkPath('walk')"
      :color="currentMode === 'walk' ? 'primary' : undefined"
    >
      {{ $t('modes.walk') }}
    </q-btn>
  </div>
</template>

<style lang="scss">
.travel-mode-bar a:not(:last-child) {
  margin-right: 8px;
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';
import { TravelMode } from 'src/utils/models';
import Config from 'src/utils/Config';
import Place from 'src/models/Place';

export default defineComponent({
  name: 'TravelModeBar',
  props: {
    currentMode: String as () => TravelMode,
    fromPlace: Place,
    toPlace: Place,
  },
  setup: function () {
    return { transitRoutingEnabled: Config.transitRoutingEnabled };
  },
  methods: {
    linkPath(mode: string): string {
      let f = (place?: Place): string => place?.urlEncodedId() ?? '_';

      return `/directions/${mode}/${f(this.toPlace)}/${f(this.fromPlace)}`;
    },
  },
});
</script>
