<template>
  <span class="route-pill-wrap">
    <span
      v-if="badge.kind == 'ride'"
      class="route-pill"
      :style="{
        borderColor: badge.routes[0]!.color,
        // Behind the vehicle, continuing the route beside it.
        backgroundColor: badge.routes[badge.routes.length - 1]!.color,
      }"
    >
      <span class="route-pill-routes">
        <template v-for="(route, idx) in badge.routes" :key="route.label">
          <span v-if="idx > 0" class="route-pill-slash" :style="slashStyle(idx)"
            >/</span
          >
          <span
            class="route-pill-route"
            :style="{ color: route.textColor, backgroundColor: route.color }"
            >{{ route.label }}</span
          >
        </template>
      </span>
      <span class="route-pill-vehicle-slot">
        <span class="route-pill-vehicle-chip">{{ badge.emoji }}</span>
      </span>
    </span>
    <span v-else class="route-pill route-pill-self-powered">
      <span class="route-pill-duration">{{ badge.minutes }}</span>
      <span class="route-pill-vehicle-slot">
        <span class="route-pill-vehicle-chip">{{ badge.emoji }}</span>
      </span>
    </span>
    <span v-if="realTime || hasAlerts" class="route-pill-badges">
      <span v-if="hasAlerts" class="route-pill-badge route-pill-badge-alert"
        ><q-icon name="warning"
      /></span>
      <span v-if="realTime" class="route-pill-badge route-pill-badge-live"
        ><q-icon name="rss_feed"
      /></span>
    </span>
  </span>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { LegBadge } from 'src/models/TripGroup';

/// A leg of a trip at a glance: the routes it rides, each in its own color, then the kind of
/// vehicle - or how long the walker or cyclist takes getting themselves there.
export default defineComponent({
  name: 'RoutePill',
  props: {
    badge: {
      type: Object as PropType<LegBadge>,
      required: true,
    },
    /// Whether the leg's times come from live data.
    realTime: Boolean,
    hasAlerts: Boolean,
  },
  methods: {
    /// The "/" before route `idx`: where the routes' colors differ, it straddles the seam between
    /// them.
    slashStyle(idx: number): Record<string, string> {
      if (this.badge.kind != 'ride') {
        return {};
      }
      const left = this.badge.routes[idx - 1]!;
      const right = this.badge.routes[idx]!;
      return {
        color: `color-mix(in srgb, ${left.textColor} 85%, transparent)`,
        background: `linear-gradient(90deg, ${left.color} 50%, ${right.color} 50%)`,
      };
    },
  },
});
</script>

<style lang="scss">
.route-pill-wrap {
  position: relative;
  display: inline-flex;
  vertical-align: middle;
}
// Tucked onto the pill's corner, so they cost the row no width.
.route-pill-badges {
  position: absolute;
  top: -6px;
  right: -7px;
  display: flex;
  // Down the pill's edge rather than along its top, so a pair covers no more of the vehicle.
  flex-direction: column;
  gap: 1px;
  pointer-events: none;
}
.route-pill-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.35);
  font-size: 9px;
  color: $hw-dark-gray;
}
.route-pill-badge-live .q-icon {
  // The glyph's weight sits in its lower left corner, so centered in a disc it looks low and left.
  transform: translate(0.5px, -0.5px);
}
.route-pill-badge-alert {
  background: #ffd54f;
  color: #000000;
}
.route-pill {
  display: inline-flex;
  align-items: stretch;
  vertical-align: middle;
  box-sizing: border-box;
  // Fixed, so a walk's lighter text sits in a pill as tall as a route's.
  height: 1.5rem;
  border-radius: 6px;
  overflow: hidden;
  border: 1.5px solid rgba(0, 0, 0, 0.55);
  line-height: 1;
  cursor: pointer;
}
// Like the vehicle markers on the map - a white disc, ringed by the pill's color - but flat, as
// the list is.
.route-pill-vehicle-slot {
  display: flex;
  align-items: center;
  padding: 0 3px 0 1px;
}
.route-pill-vehicle-chip {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #ffffff;
  font-size: 11px;
  line-height: 1;
}
.route-pill-routes {
  display: flex;
}
.route-pill-slash {
  display: flex;
  align-items: center;
  font-size: 0.9375rem;
  font-weight: 400;
}
.route-pill-route,
.route-pill-duration {
  display: flex;
  align-items: center;
  justify-content: center;
}
.route-pill-route {
  min-width: 1.75em;
  padding: 0 6px;
  text-align: center;
  font-size: 0.9375rem;
  font-weight: 700;
}
// Snug against the "/" between merged routes.
.route-pill-route:not(:first-child) {
  padding-left: 3px;
}
.route-pill-route:has(+ .route-pill-slash) {
  padding-right: 3px;
}
.route-pill-self-powered {
  background: #e4e4e4;
  border-color: #e4e4e4;
}
.route-pill-duration {
  padding: 0 4px 0 6px;
  font-size: 0.8125rem;
}
</style>
