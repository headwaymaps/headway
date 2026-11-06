<template>
  <div class="route-comparison-card">
    <table>
      <thead>
        <tr>
          <th></th>
          <th v-for="column in columns" :key="column">
            {{ COLUMN_LABELS[column] }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="router in routers" :key="router">
          <th>
            <a :href="routerLinks?.[router]" target="_blank" rel="noopener">{{
              router
            }}</a>
          </th>
          <td v-for="column in columns" :key="column">
            <button
              v-if="cell(router, column)"
              type="button"
              class="route-comparison-cell"
              @click="$emit('select', cell(router, column)!.tripIdx)"
            >
              <span
                class="route-letter"
                :style="{
                  background:
                    comparison.routes[cell(router, column)!.tripIdx]!.color,
                }"
                >{{
                  comparison.routes[cell(router, column)!.tripIdx]!.letter
                }}</span
              >
              {{ cell(router, column)!.seconds }} s
            </button>
            <span v-else>—</span>
          </td>
        </tr>
      </tbody>
    </table>
    <div class="route-comparison-footer">
      <span v-for="(trip, i) in trips" :key="i">
        <span
          class="route-letter"
          :style="{ background: comparison.routes[i]!.color }"
          >{{ comparison.routes[i]!.letter }}</span
        >
        {{ trip.distanceFormatted }}
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
import Config from 'src/utils/Config';
import type Trip from 'src/models/Trip';
import type RouteComparison from 'src/models/RouteComparison';
import type { BikeRoutePreference } from 'src/services/TravelmuxClient';

type Column = BikeRoutePreference | 'any';

const COLUMN_LABELS: Record<Column, string> = {
  quieter: 'Safer',
  faster: 'Faster',
  any: 'Time',
};

const props = defineProps<{
  trips: Trip[];
  comparison: RouteComparison;
  fittingDifferences: boolean;
}>();

defineEmits<{ select: [tripIdx: number]; 'toggle-fit': [] }>();

const plans = computed(() =>
  props.trips.flatMap((trip, tripIdx) =>
    (trip.raw.routerPlans ?? []).map((plan) => ({ ...plan, tripIdx })),
  ),
);

const routers = computed(() => [
  ...new Set(plans.value.map((plan) => plan.router)),
]);

const routerLinks = Config.routerLinks;

/// Walking has no route preferences, so its plans all fall in one column.
const columns = computed((): Column[] =>
  plans.value.some((plan) => plan.routePreferences?.length)
    ? ['quieter', 'faster']
    : ['any'],
);

function cell(router: string, column: Column) {
  const plan = plans.value.find(
    (plan) =>
      plan.router === router &&
      (column === 'any' || plan.routePreferences?.includes(column)),
  );
  return (
    plan && {
      tripIdx: plan.tripIdx,
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

.route-comparison-zoom {
  margin-left: auto;
  font: inherit;
}
</style>
