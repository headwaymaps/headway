<template>
  <q-item-label>
    {{ itinerary.startStopTimesFormatted }}
  </q-item-label>
  <q-item-label>
    <span v-for="(leg, idx) in itinerary.legs" :key="JSON.stringify(leg)">
      <span v-if="idx > 0"> → </span>
      {{ leg.shortName }}
      <sup v-if="leg.alerts.length > 0"><q-icon name="warning" /></sup>
      <sup v-if="leg.realTime" class="leg-status"
        ><q-icon name="rss_feed"
      /></sup>
    </span>
  </q-item-label>
  <q-item-label v-if="active" caption>
    {{ itinerary.walkingDistanceFormatted }}
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
  <ul v-if="itinerary.hasAlerts" class="alert-list" :hidden="!active">
    <li v-for="alert in itinerary.alerts" :key="JSON.stringify(alert)">
      ⚠️ {{ alert.headerText }}
    </li>
  </ul>
</template>
<script lang="ts">
import Itinerary from 'src/models/Itinerary';
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
  data(): { nowTime: number; itinerary: Itinerary } {
    // this cast is safe because we know that the trip is a transit trip
    const itinerary = this.trip.transitItinerary() as Itinerary;
    console.assert(itinerary);
    return {
      nowTime: Date.now(),
      itinerary,
    };
  },
  methods: {
    firstTransitLegIsRealTime(): boolean {
      return this.itinerary.firstTransitLeg?.realTime ?? false;
    },
    firstTransitLegDepartureLocation(): string | undefined {
      return this.itinerary.firstTransitLeg?.departureLocationName;
    },
    formattedDurationUntilStart(): string | undefined {
      const startTime = this.itinerary.firstTransitLeg?.startTime;
      if (!startTime) {
        return undefined;
      }
      const timeUntilStart = startTime - this.nowTime;
      if (timeUntilStart < 0) {
        return i18n.global.t('departs_$timeDuration_since_now', {
          timeDuration: formatDuration(-timeUntilStart / 1000),
        });
      } else {
        return i18n.global.t('departs_$timeDuration_from_now', {
          timeDuration: formatDuration(timeUntilStart / 1000),
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
