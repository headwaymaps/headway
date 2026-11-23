<template>
  <q-list>
    <q-item
      v-for="step in steps"
      :key="JSON.stringify(step)"
      :style="step.isDestination ? 'padding-bottom: 20px;' : ''"
      clickable
      @click="clickedStep(step)"
    >
      <q-item-section
        :top="step.isMovement"
        class="col-3 timeline-time-mode"
        :class="step.isDestination ? 'timeline-destination' : ''"
        style="text-align: right; padding-right: 6px"
      >
        <route-pill
          v-if="step.badge"
          :badge="step.badge"
          :has-alerts="step.alertGroups.length > 0"
        />
        <span v-else-if="step.realTime">
          <q-icon name="rss_feed" />
          {{ step.leftColumn }}</span
        >
        <span v-else>
          {{ step.leftColumn }}
        </span>
      </q-item-section>
      <q-item-section class="col-1">
        <div
          v-if="!step.isDestination"
          :class="step.timelineClasses"
          :style="{ textAlign: 'center', borderLeftColor: step.timelineColor }"
        >
          {{ step.timeline }}
        </div>
        <div
          v-if="step.isDestination"
          :class="step.timelineClasses"
          style="text-align: center"
        >
          <svg display="block" height="41px" width="27px" viewBox="0 0 27 41">
            <g fill-rule="nonzero">
              <g fill="#111111">
                <path
                  d="M27,13.5 C27,19.074644 20.250001,27.000002 14.75,34.500002 C14.016665,35.500004 12.983335,35.500004 12.25,34.500002 C6.7499993,27.000002 0,19.222562 0,13.5 C0,6.0441559 6.0441559,0 13.5,0 C20.955844,0 27,6.0441559 27,13.5 Z"
                ></path>
              </g>
              <g transform="translate(8.0, 8.0)">
                <circle
                  fill="#000000"
                  opacity="0.25"
                  cx="5.5"
                  cy="5.5"
                  r="5.4999962"
                ></circle>
                <circle fill="#FFFFFF" cx="5.5" cy="5.5" r="5.4999962"></circle>
              </g>
            </g>
          </svg>
        </div>
      </q-item-section>
      <q-item-section
        :top="step.isMovement"
        :class="
          ['col-8', 'timeline-description'].concat(
            step.isDestination ? 'timeline-destination' : '',
          )
        "
        >{{ step.description }}
        <div v-if="step.detail" class="timeline-detail">{{ step.detail }}</div>
        <div v-if="step.waitTimeSeconds > 60" class="timeline-wait-time">
          {{
            $t('transit_timeline_wait_for_transit_$timeDuration', {
              timeDuration: formatDuration(step.waitTimeSeconds),
            })
          }}
        </div>
        <transit-alert-list
          v-if="step.alertGroups.length > 0"
          :groups="step.alertGroups"
        />
      </q-item-section>
    </q-item>
  </q-list>
</template>

<script lang="ts">
import { defineComponent, PropType } from 'vue';
import { formatDistance, formatDuration, formatTime } from 'src/utils/format';
import { LngLat } from 'maplibre-gl';
import { focusTripStep, hideTripStepCallout } from 'src/utils/Markers';
import { i18n } from 'src/i18n/lang';
import Trip, { StepCallout, TransitAlertGroup, TripLeg } from 'src/models/Trip';
import Place from 'src/models/Place';
import { placeDisplayName } from 'src/i18n/utils';
import { DistanceUnits } from 'src/utils/models';
import TransitAlertList from 'src/components/TransitAlertList.vue';
import TripGroup, { LegBadge } from 'src/models/TripGroup';
import RoutePill from 'src/components/RoutePill.vue';

export default defineComponent({
  name: 'MultiModalSteps',
  components: { RoutePill, TransitAlertList },
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
    from: {
      type: Object as PropType<Place>,
      required: true,
    },
    to: {
      type: Object as PropType<Place>,
      required: true,
    },
  },
  data(): { steps: Step[] } {
    return {
      steps: buildSteps(this.trip, this.from, this.to),
    };
  },
  unmounted() {
    hideTripStepCallout();
  },
  methods: {
    formatTime,
    formatDuration,
    clickedStep: (step: Step): void => {
      focusTripStep(step.position, step.callout);
    },
  },
});

type Step = {
  leftColumn: string;
  timeline: string;
  timelineClasses: string[];
  timelineColor?: string;
  description: string;
  /// A quieter second line under the description.
  detail?: string;
  /// What the rider does here, labelled on the map when the step is picked.
  callout: StepCallout;
  isMovement: boolean;
  /// For a leg, how the trip list badges it.
  badge?: LegBadge;
  alertGroups: TransitAlertGroup[];
  isDestination: boolean;
  position: LngLat;
  realTime: boolean;
  waitTimeSeconds: number;
};

/// What riding or walking a leg involves, beyond the pill beside it: "toward Lynnwood" over
/// "25 min · 9 stops", or for a walk, how far.
function movementDescription(
  leg: TripLeg,
  units: DistanceUnits,
): { description: string; detail?: string } {
  const t = i18n.global.t;
  if (!leg.transitLeg) {
    // Under a tenth of a mile or kilometer, the long unit would round to nothing.
    if (leg.distanceMeters < 100) {
      const shortUnits =
        units == DistanceUnits.Miles
          ? DistanceUnits.Feet
          : DistanceUnits.Meters;
      return {
        description: formatDistance(
          leg.distanceMeters,
          DistanceUnits.Meters,
          shortUnits,
          0,
        ),
      };
    }
    return {
      description: formatDistance(
        leg.distanceMeters,
        DistanceUnits.Meters,
        units,
      ),
    };
  }
  const facts = [formatDuration(leg.durationSeconds, 'shortform')];
  const stops = leg.stopsRidden;
  if (stops !== undefined) {
    facts.push(t('transit_timeline_$n_stops', { n: stops }, stops));
  }
  const detail = facts.join(' · ');
  const headsign = leg.raw.transitLeg?.headsign;
  if (!headsign) {
    return { description: detail };
  }
  return {
    description: t('transit_timeline_toward_$headsign', { headsign }),
    detail,
  };
}

function buildSteps(trip: Trip, from: Place, to: Place): Step[] {
  const firstLeg = trip.legs[0];
  if (!firstLeg) {
    console.error('Trip has no legs');
    return [];
  }

  function pairwiseForEach(
    list: TripLeg[],
    f: (prev: TripLeg, current: TripLeg | undefined) => void,
  ): void {
    for (let idx = 0; idx < list.length; idx++) {
      f(list[idx]!, list[idx + 1]);
    }
  }

  const originStep: Step = {
    leftColumn: formatTime(firstLeg.startTime),
    timeline: '',
    timelineClasses: ['timeline-node', 'timeline-origin'],
    description: placeDisplayName(from),
    callout: firstLeg.stepCallout,
    isMovement: false,
    alertGroups: [],
    isDestination: false,
    position: firstLeg.sourceLngLat,
    realTime: firstLeg.realTime,
    waitTimeSeconds: 0,
  };

  const steps = [originStep];
  const badges = TripGroup.group([trip])[0]!.legBadges;

  pairwiseForEach(trip.legs, (prevLeg, currentLeg) => {
    let waitTimeSeconds = 0;
    if (currentLeg) {
      waitTimeSeconds =
        (currentLeg.startTime.getTime() - prevLeg.endTime.getTime()) / 1000;
    }
    steps.push({
      leftColumn: prevLeg.shortName,
      timeline: '',
      timelineClasses: ['timeline-edge', `timeline-edge-${prevLeg.mode}`],
      timelineColor: prevLeg.transitLeg ? prevLeg.routeColor : undefined,
      ...movementDescription(prevLeg, trip.preferredDistanceUnits),
      callout: prevLeg.stepCallout,
      position: prevLeg.sourceLngLat,
      isMovement: true,
      badge: badges[trip.legs.indexOf(prevLeg)]!,
      alertGroups: prevLeg.alertGroups,
      isDestination: false,
      realTime: prevLeg.realTime,
      waitTimeSeconds,
    });

    if (currentLeg) {
      steps.push({
        leftColumn: formatTime(currentLeg.startTime),
        timeline: '',
        timelineClasses: ['timeline-node'],
        description: currentLeg.sourceName,
        callout: currentLeg.stepCallout,
        position: currentLeg.sourceLngLat,
        isMovement: false,
        alertGroups: [],
        isDestination: false,
        realTime: currentLeg.realTime,
        waitTimeSeconds: 0,
      });
    } else {
      steps.push({
        leftColumn: formatTime(prevLeg.endTime),
        timeline: '',
        timelineClasses: ['timeline-node', 'timeline-destination'],
        description: placeDisplayName(to),
        callout: [i18n.global.t('leg_step_arrive')],
        position: prevLeg.destinationLngLat,
        isMovement: false,
        alertGroups: [],
        isDestination: true,
        realTime: prevLeg.realTime,
        waitTimeSeconds: 0,
      });
    }
  });

  return steps;
}
</script>

<style lang="scss">
.timeline-time-mode {
  // Hugs the timeline, and keeps a pill's badges on its own corner.
  align-items: flex-end;
}

.timeline-edge {
  position: relative;
  width: 100%;
  height: calc(100% + 20px);
  left: calc(50% - 6px);
  margin-top: -10px;
  margin-bottom: -10px;
}

.timeline-edge-walk,
.timeline-edge-bicycle {
  border-left: dashed $walkColor 6px;
}

.timeline-edge:not(.timeline-edge-walk):not(.timeline-edge-bicycle) {
  border-left: solid 6px;
}

.timeline-node {
  position: relative;
  width: 16px;
  height: 16px;
  border-radius: 8px;
  left: calc(50% - 11px);
  border: solid #888 3px;
}

.timeline-node.timeline-origin {
  border-color: black;
}

.timeline-node.timeline-destination {
  border: none;
  left: calc(50% - 15px);
}

.timeline-time-mode.timeline-destination,
.timeline-description.timeline-destination {
  margin-top: 26px;
}

.timeline-detail {
  opacity: 0.7;
}

.timeline-wait-time {
  opacity: 0.6;
}
</style>
