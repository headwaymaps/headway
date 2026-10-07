<template>
  <div class="elevation-chart">
    <svg
      v-if="profile.length > 1"
      ref="svg"
      class="elevation-svg"
      :class="{ 'elevation-svg--inert': !interactive }"
      :width="width"
      :height="height"
      :viewBox="`0 0 ${width} ${height}`"
      @pointerdown="startScrub"
      @pointermove="moveScrub"
      @pointerup="endScrub"
      @pointercancel="endScrub"
    >
      <defs>
        <linearGradient id="areaGradient" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop
            offset="0%"
            :style="`stop-color: ${FLAT_GRADE_COLOR}; stop-opacity: 0.3`"
          />
          <stop
            offset="100%"
            :style="`stop-color: ${FLAT_GRADE_COLOR}; stop-opacity: 0.1`"
          />
        </linearGradient>
      </defs>

      <!-- Shaded area under the line -->
      <path :d="areaPath" fill="url(#areaGradient)" class="elevation-area" />

      <line
        v-if="scrubPoint"
        class="scrubber"
        :x1="scrubPoint[0]"
        :x2="scrubPoint[0]"
        :y1="0"
        :y2="height"
        stroke="#111"
        stroke-opacity="0.25"
      />

      <!-- Elevation line -->
      <path
        :d="linePath"
        fill="none"
        :stroke="FLAT_GRADE_COLOR"
        stroke-width="1.5"
        class="elevation-line"
      />

      <path
        v-for="(section, idx) in steepSectionPaths"
        :key="idx"
        :d="section.path"
        fill="none"
        :stroke="section.color"
        stroke-width="2"
        stroke-linecap="round"
      />

      <g
        v-for="(annotation, idx) in steepClimbAnnotations"
        :key="`annotation-${idx}`"
        class="steep-annotation"
        @pointerdown.stop
        @click="$emit('select-steep-section', annotation.section)"
      >
        <title>{{ annotation.title }}</title>
        <text
          :x="annotation.x"
          :y="annotation.y"
          :text-anchor="annotation.anchor"
          class="steep-annotation-label"
        >
          <tspan
            v-for="(part, partIdx) in annotation.parts"
            :key="partIdx"
            :font-size="part.fontSize"
            :dx="part.dx"
            v-text="part.text"
          />
        </text>
      </g>

      <circle
        v-if="scrubPoint"
        class="scrubber"
        :cx="scrubPoint[0]"
        :cy="scrubPoint[1]"
        r="5"
        fill="white"
        stroke="#111"
        stroke-width="2"
      />
    </svg>
    <div v-else class="no-elevation-data">No elevation data available</div>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { SteepSection } from 'src/services/TravelmuxClient';
import {
  annotatedSteepClimbs,
  describeGrade,
  FLAT_GRADE_COLOR,
  gradeColor,
} from 'src/utils/grade';

interface SteepClimbAnnotation {
  x: number;
  y: number;
  anchor: 'start' | 'end';
  /// In reading order, with the arrow nearest the line.
  parts: { text: string; fontSize?: number; dx?: number }[];
  title: string;
  section: SteepSection;
}

export default defineComponent({
  name: 'ElevationChart',
  props: {
    /// `[distance, elevation]`, both in meters
    profile: {
      type: Array as PropType<[number, number][]>,
      required: true,
    },
    steepSections: {
      type: Array as PropType<SteepSection[]>,
      default: () => [],
    },
    /// Where the traveler is scrubbing, as a fraction of the way along the profile.
    scrubFraction: {
      type: Number as PropType<number | null>,
      default: null,
    },
    /// Whether the traveler can scrub the chart and pick its climbs.
    interactive: {
      type: Boolean,
      default: true,
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
  emits: ['update:scrubFraction', 'select-steep-section'],
  setup() {
    return { FLAT_GRADE_COLOR };
  },
  data(): { scrubbing: boolean } {
    return { scrubbing: false };
  },
  computed: {
    elevations(): number[] {
      return this.profile.map(([, elevation]) => elevation);
    },
    minElevation(): number {
      return Math.min(...this.elevations);
    },
    maxElevation(): number {
      return Math.max(...this.elevations);
    },
    elevationRange(): number {
      return this.maxElevation - this.minElevation;
    },
    totalDistance(): number {
      return this.profile[this.profile.length - 1]![0] - this.profile[0]![0];
    },
    /// One per profile point
    pointXs(): number[] {
      const startDistance = this.profile[0]![0];
      return this.profile.map(
        ([distance]) =>
          ((distance - startDistance) / this.totalDistance) * this.width,
      );
    },
    /// One per profile point, from 0 at the lowest to 1 at the highest
    normalizedElevations(): number[] {
      return this.profile.map(([, elevation]) =>
        this.elevationRange === 0
          ? 0.5
          : (elevation - this.minElevation) / this.elevationRange,
      );
    },
    /// `[x, y]` in chart coordinates, one per profile point
    chartPoints(): [number, number][] {
      return this.pointXs.map((x, idx) => [
        x,
        this.lineBottom - this.normalizedElevations[idx]! * this.chartHeight,
      ]);
    },
    lineBottom(): number {
      return this.height - 4;
    },
    steepSectionPaths(): { path: string; color: string }[] {
      return this.steepSections.map((section) => {
        const points = this.pointsIn(section);
        return {
          path: `M ${points.map(([x, y]) => `${x},${y}`).join(' L ')}`,
          color: gradeColor(section.averageGrade),
        };
      });
    },
    chartHeight(): number {
      return this.lineBottom - this.lineTop;
    },
    /// Room for the scrubber's dot at the highest point.
    lineTop(): number {
      return 6;
    },
    /// A label just above each climb's midpoint, where selecting the climb puts the scrubber's dot.
    /// It reaches back over the lower part of the climb, unless that runs off the chart.
    steepClimbAnnotations(): SteepClimbAnnotation[] {
      const labelWidth = 32;
      // the label ends just past the dot's center, so its arrow sits over the dot
      const overhang = 4;
      const capHeight = 7;
      const dotRadius = 5;
      return annotatedSteepClimbs(this.steepSections).map((section) => {
        const points = this.pointsIn(section);
        const xs = points.map(([x]) => x);
        const centerX = (Math.min(...xs) + Math.max(...xs)) / 2;
        const [midX, midY] = points.reduce((nearest, point) =>
          Math.abs(point[0] - centerX) < Math.abs(nearest[0] - centerX)
            ? point
            : nearest,
        );
        const fitsLeft = midX + overhang - labelWidth >= 0;
        const arrow = { text: '↗' };
        const number = { text: `${Math.round(section.averageGrade * 100)}` };
        const percentSign = { text: '%', fontSize: 7 };
        return {
          x: fitsLeft ? midX + overhang : midX - overhang,
          y: Math.max(midY - dotRadius - 2, capHeight),
          anchor: fitsLeft ? 'end' : 'start',
          parts: fitsLeft
            ? [number, percentSign, { ...arrow, dx: 3 }]
            : [arrow, { ...number, dx: 3 }, percentSign],
          title: describeGrade(section.averageGrade),
          section,
        };
      });
    },
    scrubPoint(): [number, number] | undefined {
      if (this.scrubFraction === null) {
        return undefined;
      }
      const x = this.scrubFraction * this.width;
      const points = this.chartPoints;
      const after = points.findIndex(([pointX]) => pointX >= x);
      if (after <= 0) {
        return points[Math.max(after, 0)];
      }
      const [x0, y0] = points[after - 1]!;
      const [x1, y1] = points[after]!;
      const t = x1 === x0 ? 0 : (x - x0) / (x1 - x0);
      return [x, y0 + t * (y1 - y0)];
    },
    linePath(): string {
      const points = this.chartPoints.map(([x, y]) => `${x},${y}`);
      return `M ${points.join(' L ')}`;
    },
    areaPath(): string {
      const points = this.chartPoints.map(([x, y]) => `${x},${y}`);

      return `M 0,${this.height} L ${points.join(' L ')} L ${this.width},${this.height} Z`;
    },
  },
  methods: {
    /// The chart points along `section`.
    pointsIn(section: SteepSection): [number, number][] {
      return this.chartPoints.filter((_, idx) => {
        const distance = this.profile[idx]![0];
        return distance >= section.startMeters && distance <= section.endMeters;
      });
    },
    startScrub(event: PointerEvent) {
      this.scrubbing = true;
      (event.currentTarget as Element).setPointerCapture(event.pointerId);
      this.scrubTo(event);
    },
    moveScrub(event: PointerEvent) {
      if (this.scrubbing) {
        this.scrubTo(event);
      }
    },
    endScrub() {
      this.scrubbing = false;
    },
    scrubTo(event: PointerEvent) {
      const svg = this.$refs.svg as SVGSVGElement;
      const rect = svg.getBoundingClientRect();
      const fraction = (event.clientX - rect.left) / rect.width;
      this.$emit('update:scrubFraction', Math.min(Math.max(fraction, 0), 1));
    },
  },
});
</script>

<style lang="scss" scoped>
.elevation-chart {
  // no side padding, so the line lines up with the text around the chart
  padding: 8px 0;
  background: white;
  border-radius: 4px;
  margin: 8px 0;
}

.elevation-svg {
  // the scrubber's dot hangs past the ends of the line
  overflow: visible;
  cursor: crosshair;
  // dragging the scrubber shouldn't scroll the page
  touch-action: none;
}

.elevation-svg--inert {
  pointer-events: none;
}

.steep-annotation {
  cursor: pointer;
}

.steep-annotation-label {
  font-size: 10px;
  font-weight: 700;
  fill: $climb-color;
  stroke: white;
  stroke-width: 2px;
  stroke-linejoin: round;
  paint-order: stroke;
}

.scrubber {
  pointer-events: none;
}

.elevation-line {
  transition: stroke-width 0.2s;

  &:hover {
    stroke-width: 2;
  }
}

.elevation-area {
  transition: opacity 0.2s;
}

.no-elevation-data {
  font-size: 12px;
  color: #888;
  text-align: center;
  padding: 20px;
}
</style>
