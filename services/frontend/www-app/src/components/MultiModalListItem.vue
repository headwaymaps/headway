<template>
  <q-item-label>
    {{ trip.startStopTimesFormatted }}
  </q-item-label>
  <q-item-label>
    <span v-for="(leg, idx) in trip.legs" :key="JSON.stringify(leg)">
      <span v-if="idx > 0"> → </span>
      {{ leg.shortName }}
      <sup v-if="leg.alerts.length > 0"><q-icon name="warning" /></sup>
      <sup v-if="leg.realTime" class="leg-status"
        ><q-icon name="rss_feed"
      /></sup>
    </span>
  </q-item-label>
  <q-item-label v-if="active" caption>
    {{ trip.walkingDistanceFormatted }}
  </q-item-label>
  <div v-if="formattedDurationUntilStart() !== undefined">
    <q-icon
      v-if="firstTransitLegIsRealTime()"
      name="rss_feed"
      style="margin-right: 4px"
    />
    <span class="real-time-departure-time">
      {{ formattedDurationUntilStart() }}&nbsp;
    </span>
    <span
      v-if="firstTransitLegDepartureLocation()"
      class="real-time-departure-location"
    >
      {{
        $t('departs_at_$location', {
          location: firstTransitLegDepartureLocation(),
        })
      }}
    </span>
  </div>
  <ul v-if="trip.hasAlerts" class="alert-list" :hidden="!active">
    <li v-for="alert in trip.alerts" :key="JSON.stringify(alert)">
      ⚠️ {{ alert.headerText }}
    </li>
  </ul>
</template>
<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { formatDuration } from 'src/utils/format';
import { i18n } from 'src/i18n/lang';
import Trip from 'src/models/Trip';

export default defineComponent({
  name: 'MultiModalListItem',
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    active: {
      type: Boolean,
      required: true,
    },
  },
  data(): { nowTime: number } {
    return {
      nowTime: Date.now(),
    };
  },
  methods: {
    firstTransitLegIsRealTime(): boolean {
      return this.trip.firstTransitLeg?.realTime ?? false;
    },
    firstTransitLegDepartureLocation(): string | undefined {
      return this.trip.firstTransitLeg?.departureLocationName;
    },
    formattedDurationUntilStart(): string | undefined {
      const startTime = this.trip.firstTransitLeg?.startTime;
      if (!startTime) {
        return undefined;
      }
      const secondsUntilStart = (startTime.getTime() - this.nowTime) / 1000;
      if (secondsUntilStart < 0) {
        return i18n.global.t('departs_$timeDuration_since_now', {
          timeDuration: formatDuration(-secondsUntilStart),
        });
      } else {
        return i18n.global.t('departs_$timeDuration_from_now', {
          timeDuration: formatDuration(secondsUntilStart),
        });
      }
    },
  },
});
</script>

<style lang="scss">
.alert-list {
  list-style: none;
  padding: 0;
}
.real-time-departure-location {
  opacity: 0.8;
}
.real-time-departure-time {
  font-weight: 500;
}
</style>
