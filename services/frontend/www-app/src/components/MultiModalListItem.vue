<template>
  <q-item-label>
    {{ trip.startStopTimesFormatted() }}
  </q-item-label>
  <q-item-label>
    <span v-for="(leg, idx) in trip.legs" v-bind:key="JSON.stringify(leg)">
      <span v-if="idx > 0"> → </span>
      {{ leg.shortName }}
      <sup v-if="leg.alerts.length > 0"><q-icon name="warning" /></sup>
      <sup v-if="leg.realTime" class="leg-status"
        ><q-icon name="rss_feed"
      /></sup>
    </span>
  </q-item-label>
  <q-item-label caption :hidden="!active">
    {{
      $t('walk_distance', {
        preformattedDistance: trip.walkingDistanceFormatted(),
      })
    }}
  </q-item-label>
  <div v-if="trip.firstTransitLeg?.realTime">
    <q-icon name="rss_feed" style="margin-right: 4px" />
    <span class="real-time-departure-time">
      {{
        $t('departs_$timeDuration_from_now', {
          timeDuration: formatDuration(
            (trip.firstTransitLeg!.startTime - nowTime) / 1000,
            'shortform'
          ),
        })
      }}&nbsp;
    </span>
    <span class="real-time-departure-location">
      {{
        $t('departs_at_$location', {
          location: 'John & 5th',
        })
      }}
    </span>
  </div>
  <ul class="alert-list" :hidden="!active" v-if="trip.hasAlerts">
    <li v-for="alert in trip.alerts" v-bind:key="JSON.stringify(alert)">
      ⚠️ {{ alert.headerText }}
    </li>
  </ul>
</template>
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

<script lang="ts">
import Itinerary from 'src/models/Itinerary';
import { defineComponent, PropType } from 'vue';
import { formatDuration } from 'src/utils/format';

export default defineComponent({
  name: 'MultiModalListItem',
  props: {
    trip: {
      type: Object as PropType<Itinerary>,
      required: true,
    },
    active: {
      type: Boolean,
      required: true,
    },
    earliestStart: {
      type: Number,
      required: true,
    },
    latestArrival: {
      type: Number,
      required: true,
    },
  },
  data(): { nowTime: number } {
    return { nowTime: Date.now() };
  },
  methods: {
    formatDuration,
  },
});
</script>
