<template>
  <div>
    <div v-if="elevation" class="q-px-md steps-elevation-chart">
      <elevation-chart
        v-model:scrub-fraction="scrubFraction"
        :profile="elevation.profile"
        :steep-sections="elevation.steepSections"
        :width="300"
        :height="100"
        @select-steep-section="selectSteepClimb"
      />
    </div>
    <q-expansion-item
      v-if="elevation"
      dense
      header-class="climbs-header"
      :hide-expand-icon="steepClimbs.length === 0"
    >
      <template #header>
        <q-item-section class="climbs-summary">
          <div>
            {{ $t('elevation_climbs') }}
            <elevation-totals
              :climb-meters="elevation.totalClimbMeters"
              :fall-meters="elevation.totalFallMeters"
              :distance-units="trip.preferredDistanceUnits"
            />
          </div>
        </q-item-section>
      </template>
      <q-list dense>
        <q-item
          v-for="(section, idx) in steepClimbs"
          :key="idx"
          clickable
          @click="selectSteepClimb(section)"
        >
          <q-item-section avatar>
            <span
              class="steep-section-grade"
              :style="{ color: gradeColor(section.averageGrade) }"
              >{{ formatClimbGrade(section.averageGrade) }}</span
            >
          </q-item-section>
          <q-item-section>
            <q-item-label>
              {{ formatSteepSection(section) }}
            </q-item-label>
          </q-item-section>
        </q-item>
      </q-list>
    </q-expansion-item>
    <q-list>
      <q-item
        v-for="maneuver in nonTransitLeg.maneuvers"
        :key="JSON.stringify(maneuver)"
        class="maneuver"
        active-class="list-item--selected"
        :active="selectedManeuver === maneuver"
        clickable
        @click="clickedManeuver(maneuver)"
      >
        <q-item-section avatar>
          <q-icon :name="valhallaTypeToIcon(maneuver.type)" />
        </q-item-section>
        <q-item-section>
          <q-item-label>
            {{ maneuver.instruction }}
          </q-item-label>
          <q-item-label caption>
            {{ maneuver.verbalPostTransitionInstruction }}
          </q-item-label>
        </q-item-section>
      </q-item>
    </q-list>
  </div>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { valhallaTypeToIcon } from 'src/services/ValhallaAPI';
import { getBaseMap } from './BaseMap.vue';
import Trip from 'src/models/Trip';
import {
  NonTransitLeg,
  SteepSection,
  TravelmuxManeuver,
} from 'src/services/TravelmuxClient';
import Markers from 'src/utils/Markers';
import { useLegElevation } from 'src/composables/useLegElevation';
import ElevationChart from './ElevationChart.vue';
import ElevationTotals from './ElevationTotals.vue';
import { formatDistance, getElevationUnits } from 'src/utils/format';
import { DistanceUnits } from 'src/utils/models';
import { formatClimbGrade, gradeColor } from 'src/utils/grade';

export default defineComponent({
  name: 'SingleModeSteps',
  components: {
    ElevationChart,
    ElevationTotals,
  },
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
  },
  setup(props) {
    return useLegElevation(props.trip.legs[0]!);
  },
  data(): {
    selectedManeuver: TravelmuxManeuver | undefined;
    geometry: GeoJSON.LineString;
    nonTransitLeg: NonTransitLeg;
  } {
    // this cast is safe because we know that the trip is a non-transit trip
    const nonTransitLeg = this.trip.legs[0]?.raw.nonTransitLeg as NonTransitLeg;
    console.assert(nonTransitLeg);
    return {
      selectedManeuver: undefined,
      nonTransitLeg,
      geometry: this.trip.legs[0]!.geometry,
    };
  },
  computed: {
    steepClimbs(): SteepSection[] {
      return (this.elevation?.steepSections ?? []).filter(
        (section) => section.averageGrade > 0,
      );
    },
  },
  methods: {
    valhallaTypeToIcon,
    formatClimbGrade,
    gradeColor,
    formatSteepSection(section: SteepSection): string {
      const length = formatDistance(
        section.endMeters - section.startMeters,
        DistanceUnits.Meters,
        getElevationUnits(this.trip.preferredDistanceUnits),
        0,
      );
      const steepest = `${Math.round(Math.abs(section.maxGrade) * 100)}%`;
      return section.streetName
        ? this.$t('steep_section_$length_$street_$steepest', {
            length,
            street: section.streetName,
            steepest,
          })
        : this.$t('steep_section_$length_$steepest', { length, steepest });
    },
    clickedManeuver: function (maneuver: TravelmuxManeuver) {
      this.selectedManeuver = maneuver;
      const baseMap = getBaseMap();
      if (baseMap) {
        baseMap.flyTo(maneuver.startPoint, { zoom: 16 });

        // Add a marker for the maneuver location
        const icon = valhallaTypeToIcon(maneuver.type);
        const marker = Markers.maneuver(icon, maneuver.bearingBefore || 0);
        marker.setRotationAlignment('map');
        marker.setLngLat(maneuver.startPoint);
        baseMap.pushMarker('selected-maneuver', marker);
      }
    },
  },
});
</script>

<style lang="scss">
.steps-elevation-chart {
  // the climbs header below tucks up against the chart, past its padding and margin
  margin-bottom: -16px;
}

.q-item.climbs-header {
  min-height: 0;
  padding-top: 0;
  padding-bottom: 6px;
}

.climbs-summary {
  font-size: 13px;

  .elevation-totals {
    font-size: 13px;
    margin-left: 4px;
  }
}

.maneuver {
  padding-top: 10px;
  padding-bottom: 10px;
  border-bottom: solid 1px #ddd;
}
</style>
