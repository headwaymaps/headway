<template>
  <div class="route-comparison-card">
    <table>
      <thead>
        <tr>
          <th>
            <button type="button" class="route-comparison-help">
              <q-icon name="info_outline" size="16px" />
              <span>Help</span>
              <q-menu class="route-comparison-help-menu">
                <p v-for="(paragraph, i) in HELP" :key="i">{{ paragraph }}</p>
              </q-menu>
            </button>
          </th>
          <th
            v-for="column in columns"
            :key="column"
            :title="COLUMN_HINTS[column]"
          >
            {{ COLUMN_LABELS[column] }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="router in routers" :key="router.name">
          <th>
            <a :href="router.link" target="_blank" rel="noopener">{{
              router.name
            }}</a>
          </th>
          <td v-for="column in columns" :key="column">
            <button
              v-if="cell(router.name, column)"
              type="button"
              class="route-comparison-cell"
              @click="$emit('select', cell(router.name, column)!.routeIdx)"
            >
              <span
                class="route-letter"
                :style="{
                  background:
                    comparison.routes[cell(router.name, column)!.routeIdx]!
                      .color,
                }"
                >{{
                  comparison.routes[cell(router.name, column)!.routeIdx]!.letter
                }}</span
              >
              {{ cell(router.name, column)!.seconds }} s
            </button>
            <span v-else>—</span>
          </td>
        </tr>
      </tbody>
    </table>
    <div class="route-comparison-footer">
      <span v-for="(route, i) in routes" :key="i">
        <span
          class="route-letter"
          :style="{ background: comparison.routes[i]!.color }"
          >{{ comparison.routes[i]!.letter }}</span
        >
        {{ route.trip.distanceFormatted }}
      </span>
      <button
        v-if="comparison.differenceBounds"
        type="button"
        class="route-comparison-zoom"
        @click="$emit('toggle-fit')"
      >
        {{ fittingDifferences ? 'Whole trip' : 'Zoom to differences' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { ComparedRoute, ComparedRouter } from 'src/models/ComparedRoutes';
import type RouteComparison from 'src/models/RouteComparison';
import type { BikeRoutePreference } from 'src/services/TravelmuxClient';

type Column = BikeRoutePreference | 'any';

const COLUMN_LABELS: Record<Column, string> = {
  quieter: 'Safer',
  faster: 'Faster',
  any: 'Time',
};

const HELP = [
  'Each row is an OTP build, linked to its commit, running its own graph built from the same ' +
    'OSM, elevation and GTFS inputs.',
  'For bikes, each build is asked twice: once optimizing for SAFE_STREETS (Safer) and once for ' +
    'SHORTEST_DURATION (Faster). Routes from different builds that are nearly identical are ' +
    'merged and share a letter, so two cells with the same letter may differ slightly. Each cell ' +
    'shows the duration that build gave its route.',
  'On the map, each route gets its own color, with shared portions colored gray. Pick a trip ' +
    'from the corpus, or plan your own.',
];

const COLUMN_HINTS: Partial<Record<Column, string>> = {
  quieter:
    'OTP optimization SAFE_STREETS: favors bike lanes, paths and quiet streets, even if slower',
  faster: 'OTP optimization SHORTEST_DURATION: the quickest route',
};

const props = defineProps<{
  routers: ComparedRouter[];
  routes: ComparedRoute[];
  comparison: RouteComparison;
  fittingDifferences: boolean;
}>();

defineEmits<{ select: [routeIdx: number]; 'toggle-fit': [] }>();

const plans = computed(() =>
  props.routes.flatMap((route, routeIdx) =>
    route.plans.map((plan) => ({ ...plan, routeIdx })),
  ),
);

/// Walking has no route preferences, so its plans all fall in one column.
const columns = computed((): Column[] =>
  plans.value.some((plan) => plan.routePreferences.length)
    ? ['quieter', 'faster']
    : ['any'],
);

function cell(router: string, column: Column) {
  const plan = plans.value.find(
    (plan) =>
      plan.router === router &&
      (column === 'any' || plan.routePreferences.includes(column)),
  );
  return (
    plan && {
      routeIdx: plan.routeIdx,
      seconds: Math.round(plan.durationSeconds),
    }
  );
}
</script>

<style lang="scss">
.route-letter {
  display: inline-block;
  flex: none;
  align-self: flex-start;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  color: white;
  font-size: 12px;
  font-weight: 700;
  line-height: 20px;
  text-align: center;
}

.route-comparison-help-menu {
  max-width: 360px;
  padding: 10px 14px;
  font-size: 13px;

  p:last-child {
    margin-bottom: 0;
  }
}

.route-letter-marker {
  border: 2px solid white;
  box-sizing: content-box;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
  pointer-events: none;
}
</style>

<style lang="scss" scoped>
.route-comparison-card {
  position: fixed;
  top: 12px;
  right: 56px;
  z-index: 10;
  padding: 8px 10px;
  background: white;
  border-radius: 6px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  font-size: 13px;

  @media (max-width: 799px) {
    position: static;
    margin: 0 8px 8px;
    box-shadow: none;
    border: 1px solid rgba(0, 0, 0, 0.15);
  }

  a {
    color: inherit;
  }

  th {
    padding: 2px 8px;
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
  }

  th[title] {
    cursor: help;
  }

  td {
    padding: 2px 8px;
  }
}

.route-comparison-cell {
  display: flex;
  gap: 6px;
  align-items: center;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
  font: inherit;
  white-space: nowrap;
}

.route-comparison-footer {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-top: 6px;
  padding: 6px 8px 0;
  border-top: 1px solid rgba(0, 0, 0, 0.15);
  white-space: nowrap;
}

.route-comparison-help {
  display: flex;
  gap: 2px;
  align-items: center;
  padding: 0;
  border: none;
  background: none;
  color: $primary;
  cursor: pointer;
  font: inherit;
  font-weight: 400;

  span {
    text-decoration: underline;
  }
}

.route-comparison-zoom {
  margin-left: auto;
  font: inherit;
}
</style>
