<template>
  <span class="elevation-totals">
    <span v-if="climbMeters > 0" class="climb-total"
      >↗ {{ formatElevation(climbMeters) }}</span
    >
    <span v-if="fallMeters > 0" class="fall-total"
      >↘ {{ formatElevation(fallMeters) }}</span
    >
  </span>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { DistanceUnits } from 'src/utils/models';
import { formatDistance, getElevationUnits } from 'src/utils/format';

/// How far a route climbs and descends in all, e.g. "↗ 120 ft ↘ 80 ft".
export default defineComponent({
  name: 'ElevationTotals',
  props: {
    climbMeters: {
      type: Number,
      required: true,
    },
    fallMeters: {
      type: Number,
      required: true,
    },
    distanceUnits: {
      type: String as PropType<DistanceUnits>,
      required: true,
    },
  },
  methods: {
    formatElevation(meters: number): string {
      return formatDistance(
        meters,
        DistanceUnits.Meters,
        getElevationUnits(this.distanceUnits),
        0,
      );
    },
  },
});
</script>

<style lang="scss" scoped>
.elevation-totals {
  display: inline-flex;
  gap: 8px;
  font-size: 11px;
}

.climb-total {
  color: $climb-color;
  font-weight: 500;
}

.fall-total {
  color: #3182ce;
  font-weight: 500;
}
</style>
