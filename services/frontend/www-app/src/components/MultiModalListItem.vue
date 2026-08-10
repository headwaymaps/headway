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
    <li v-for="(group, idx) in trip.alertGroups" :key="groupKey(group, idx)">
      <div
        class="alert-header"
        :class="{ 'alert-header-expandable': isExpandable(group) }"
        @click.stop="isExpandable(group) && toggleGroup(idx)"
      >
        ⚠️ {{ groupLabel(group) }}
        <span v-if="group.alerts.length > 1" class="alert-count">
          {{ group.alerts.length }}
        </span>
        <q-icon
          v-if="isExpandable(group)"
          :name="isExpanded(idx) ? 'expand_less' : 'expand_more'"
        />
      </div>
      <ul v-if="isExpandable(group) && isExpanded(idx)" class="alert-details">
        <li v-for="alert in group.alerts" :key="alert.descriptionText">
          {{ alert.descriptionText.trim() }}
          <a
            v-if="alert.url"
            :href="alert.url"
            target="_blank"
            rel="noopener noreferrer"
            @click.stop
            >{{ $t('transit_alert_more_info') }}</a
          >
        </li>
      </ul>
    </li>
  </ul>
</template>
<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { formatDuration } from 'src/utils/format';
import { i18n } from 'src/i18n/lang';
import Trip, { TransitAlertGroup } from 'src/models/Trip';

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
  data(): { nowTime: number; expandedAlertGroups: number[] } {
    return {
      nowTime: Date.now(),
      expandedAlertGroups: [],
    };
  },
  methods: {
    // A group is only worth expanding when its header stands in for something else - a lone
    // headerless alert already shows its full text as the label.
    isExpandable(group: TransitAlertGroup): boolean {
      return group.headerText !== undefined;
    },
    groupLabel(group: TransitAlertGroup): string {
      return (
        group.headerText ??
        group.alerts[0]?.descriptionText ??
        ''
      ).trim();
    },
    groupKey(group: TransitAlertGroup, idx: number): string {
      return `${idx}-${group.headerText ?? ''}`;
    },
    isExpanded(idx: number): boolean {
      return this.expandedAlertGroups.includes(idx);
    },
    toggleGroup(idx: number) {
      if (this.isExpanded(idx)) {
        this.expandedAlertGroups = this.expandedAlertGroups.filter(
          (expanded) => expanded !== idx,
        );
      } else {
        this.expandedAlertGroups = [...this.expandedAlertGroups, idx];
      }
    },
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
.alert-header-expandable {
  cursor: pointer;
}
.alert-count {
  opacity: 0.7;
  font-size: 0.85em;
  border: 1px solid currentcolor;
  border-radius: 999px;
  padding: 0 5px;
}
.alert-details {
  list-style: none;
  padding: 0 0 0 1.5em;
  opacity: 0.9;
  font-size: 0.9em;

  li + li {
    margin-top: 6px;
  }
}
.real-time-departure-location {
  opacity: 0.8;
}
.real-time-departure-time {
  font-weight: 500;
}
</style>
