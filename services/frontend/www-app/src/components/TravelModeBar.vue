<template>
  <div class="travel-mode-bar">
    <q-btn
      v-for="option in options"
      :key="option.mode"
      :icon="option.icon"
      unelevated
      :ripple="false"
      size="sm"
      :to="linkPath(option.mode)"
      class="travel-mode"
      :class="{ 'travel-mode--selected': currentMode === option.mode }"
    >
      {{ $t(option.label) }}
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
  computed: {
    options(): { mode: TravelMode; icon: string; label: string }[] {
      const transit = {
        mode: TravelMode.Transit,
        icon: 'directions_bus',
        label: 'modes.transit',
      };
      return [
        ...(Config.transitRoutingEnabled ? [transit] : []),
        {
          mode: TravelMode.Drive,
          icon: 'directions_car',
          label: 'modes.drive',
        },
        { mode: TravelMode.Bike, icon: 'directions_bike', label: 'modes.bike' },
        { mode: TravelMode.Walk, icon: 'directions_walk', label: 'modes.walk' },
      ];
    },
  },
  methods: {
    linkPath(mode: TravelMode): string {
      const f = (place?: Place): string => place?.urlEncodedId() ?? '_';

      return `/directions/${mode}/${f(this.toPlace)}/${f(this.fromPlace)}`;
    },
  },
});
</script>

<style lang="scss">
// A segmented control: one track, with the selected mode on a white thumb.
.travel-mode-bar {
  display: flex;
  padding: 2px;
  border-radius: 9px;
  background: $hw-search-field-background;
}

.q-btn.travel-mode {
  flex: 1;
  padding: 4px 2px;
  border-radius: 7px;
  color: $hw-field-label;

  .q-btn__content {
    flex-wrap: nowrap;
  }

  .q-icon {
    margin-right: 4px;
  }
}

.q-btn.travel-mode--selected {
  background: white;
  color: $dark;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
}
</style>
