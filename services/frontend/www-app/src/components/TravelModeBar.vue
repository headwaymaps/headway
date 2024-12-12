<template>
  <div class="travel-mode-bar">
    <q-btn
      v-if="transitRoutingEnabled"
      icon="directions_bus"
      unelevated
      rounded
      :ripple="false"
      size="sm"
      :to="linkPath('transit')"
      :color="currentMode === TravelMode.Transit ? 'primary' : undefined"
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
      :color="currentMode === TravelMode.Drive ? 'primary' : undefined"
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
      :color="currentMode === TravelMode.Bike ? 'primary' : undefined"
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
      :color="currentMode === TravelMode.Walk ? 'primary' : undefined"
    >
      {{ $t('modes.walk') }}
    </q-btn>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { TravelMode } from 'src/utils/models';
import Config from 'src/utils/Config';
import Place from 'src/models/Place';

export default defineComponent({
  name: 'TravelModeBar',
  props: {
    currentMode: {
      type: String as () => TravelMode,
      default: undefined,
    },
    fromPlace: {
      type: Place,
      default: undefined,
    },
    toPlace: {
      type: Place,
      default: undefined,
    },
  },
  setup: function () {
    return { transitRoutingEnabled: Config.transitRoutingEnabled };
  },
  data: () => ({
    TravelMode,
  }),
  methods: {
    linkPath(mode: string): string {
      const f = (place?: Place): string => place?.urlEncodedId() ?? '_';

      return `/directions/${mode}/${f(this.toPlace)}/${f(this.fromPlace)}`;
    },
  },
});
</script>

<style lang="scss">
.travel-mode-bar a:not(:last-child) {
  margin-right: 8px;
}
</style>
