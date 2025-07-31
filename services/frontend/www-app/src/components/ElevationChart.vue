<template>
  <div class="elevation-chart">
    <div class="elevation-chart-header">
      <div
        v-if="totalClimbMeters > 0 || totalFallMeters > 0"
        class="elevation-stats"
      >
        <span v-if="totalClimbMeters > 0" class="climb-stat"
          >↗ {{ formattedClimb }}</span
        >
        <span v-if="totalFallMeters > 0" class="fall-stat"
          >↘ {{ formattedFall }}</span
        >
      </div>
    </div>
    <svg
      v-if="elevations.length > 0"
      :width="width"
      :height="height"
      :viewBox="`0 0 ${width} ${height}`"
    >
      <!-- Background grid lines -->
      <defs>
        <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
          <path
            d="M 20 0 L 0 0 0 20"
            fill="none"
            stroke="#f0f0f0"
            stroke-width="1"
          />
        </pattern>
        <linearGradient id="areaGradient" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" style="stop-color: #2196f3; stop-opacity: 0.3" />
          <stop offset="100%" style="stop-color: #2196f3; stop-opacity: 0.1" />
        </linearGradient>
      </defs>
      <rect width="100%" height="100%" fill="url(#grid)" />

      <!-- Shaded area under the line -->
      <path :d="areaPath" fill="url(#areaGradient)" class="elevation-area" />

      <!-- Elevation line -->
      <path
        :d="linePath"
        fill="none"
        stroke="#2196F3"
        stroke-width="2"
        class="elevation-line"
      />

      <!-- Y-axis labels -->
      <g class="y-axis">
        <text :x="5" :y="margin" class="axis-label">
          {{ formattedMaxElevation }}
        </text>
        <text :x="5" :y="height - 5" class="axis-label">
          {{ formattedMinElevation }}
        </text>
      </g>
    </svg>
    <div v-else class="no-elevation-data">No elevation data available</div>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { DistanceUnits } from 'src/utils/models';
import { formatDistance, getElevationUnits } from 'src/utils/format';

export default defineComponent({
  name: 'ElevationChart',
  props: {
    elevations: {
      type: Array as PropType<number[]>,
      required: true,
    },
    totalClimbMeters: {
      type: Number,
      default: 0,
    },
    totalFallMeters: {
      type: Number,
      default: 0,
    },
    distanceUnits: {
      type: String as PropType<DistanceUnits>,
      default: DistanceUnits.Meters,
    },
    width: {
      type: Number,
      default: 300,
    },
    height: {
      type: Number,
      default: 100,
    },
  },
  computed: {
    minElevation(): number {
      return this.elevations.length > 0 ? Math.min(...this.elevations) : 0;
    },
    maxElevation(): number {
      return this.elevations.length > 0 ? Math.max(...this.elevations) : 0;
    },
    elevationRange(): number {
      return this.maxElevation - this.minElevation;
    },
    normalizedElevations(): number[] {
      if (this.elevationRange === 0) {
        return this.elevations.map(() => 0.5);
      }
      return this.elevations.map(
        (elevation) => (elevation - this.minElevation) / this.elevationRange,
      );
    },
    chartWidth(): number {
      return this.width - 2 * this.margin;
    },
    chartHeight(): number {
      return this.height - 2 * this.margin;
    },
    margin(): number {
      return 15;
    },
    linePath(): string {
      if (this.elevations.length === 0) return '';

      const points = this.normalizedElevations.map((elevation, index) => {
        const x =
          this.margin +
          (index / (this.elevations.length - 1)) * this.chartWidth;
        const y = this.height - this.margin - elevation * this.chartHeight;
        return `${x},${y}`;
      });

      return `M ${points.join(' L ')}`;
    },
    areaPath(): string {
      if (this.elevations.length === 0) return '';

      const points = this.normalizedElevations.map((elevation, index) => {
        const x =
          this.margin +
          (index / (this.elevations.length - 1)) * this.chartWidth;
        const y = this.height - this.margin - elevation * this.chartHeight;
        return `${x},${y}`;
      });

      const firstX = this.margin;
      const lastX = this.margin + this.chartWidth;
      const bottomY = this.height - this.margin;

      return `M ${firstX},${bottomY} L ${points.join(' L ')} L ${lastX},${bottomY} Z`;
    },
    elevationUnits(): DistanceUnits {
      return getElevationUnits(this.distanceUnits);
    },
    formattedMaxElevation(): string {
      return formatDistance(
        this.maxElevation,
        DistanceUnits.Meters,
        this.elevationUnits,
        0,
      );
    },
    formattedMinElevation(): string {
      return formatDistance(
        this.minElevation,
        DistanceUnits.Meters,
        this.elevationUnits,
        0,
      );
    },
    formattedClimb(): string {
      return formatDistance(
        this.totalClimbMeters,
        DistanceUnits.Meters,
        this.elevationUnits,
        0,
      );
    },
    formattedFall(): string {
      return formatDistance(
        this.totalFallMeters,
        DistanceUnits.Meters,
        this.elevationUnits,
        0,
      );
    },
  },
});
</script>

<style lang="scss" scoped>
.elevation-chart {
  padding: 8px;
  background: white;
  border-radius: 4px;
  margin: 8px 0;
}

.elevation-chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.elevation-chart-title {
  font-size: 12px;
  font-weight: 600;
  color: #666;
}

.elevation-stats {
  display: flex;
  gap: 8px;
  font-size: 11px;
}

.climb-stat {
  color: #e53e3e;
  font-weight: 500;
}

.fall-stat {
  color: #3182ce;
  font-weight: 500;
}

.elevation-line {
  transition: stroke-width 0.2s;

  &:hover {
    stroke-width: 3;
  }
}

.elevation-area {
  transition: opacity 0.2s;
}

.axis-label {
  font-size: 10px;
  fill: #666;
}

.no-elevation-data {
  font-size: 12px;
  color: #888;
  text-align: center;
  padding: 20px;
}
</style>
