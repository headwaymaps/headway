<template>
  <q-list>
    <q-item
      v-for="step in steps"
      :key="JSON.stringify(step)"
      :style="step.isDestination ? 'padding-bottom: 20px;' : ''"
      :clickable="!step.isMovement"
      @click="clickedStep(step)"
    >
      <q-item-section
        :top="step.isMovement"
        class="col-3 timeline-time-mode"
        :class="step.isDestination ? 'timeline-destination' : ''"
        style="text-align: right; padding-right: 6px"
      >
        <span v-if="!step.isMovement && step.realTime">
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
          style="text-align: center"
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
        <div v-if="step.waitTime > 60" class="timeline-wait-time">
          {{
            $t('transit_timeline_wait_for_transit_$timeDuration', {
              timeDuration: formatDuration(step.waitTime / 1000),
            })
          }}
        </div>
      </q-item-section>
    </q-item>
  </q-list>
</template>

<script lang="ts">
import Itinerary, { ItineraryLeg } from 'src/models/Itinerary';
import { defineComponent, PropType } from 'vue';
import { formatDuration, formatTime } from 'src/utils/format';
import { LngLat } from 'maplibre-gl';
import { getBaseMap } from 'src/components/BaseMap.vue';
import Trip from 'src/models/Trip';

export default defineComponent({
  name: 'MultiModalSteps',
  props: {
    trip: {
      type: Object as PropType<Trip>,
      required: true,
    },
  },
  data(): { steps: Step[]; itinerary: Itinerary } {
    // this cast is safe because we know that the trip is a transit trip
    const itinerary = this.trip.transitItinerary() as Itinerary;
    console.assert(itinerary);
    return {
      steps: buildSteps(itinerary),
      itinerary,
    };
  },
  methods: {
    formatTime,
    formatDuration,
    clickedStep: (step: Step): void => {
      getBaseMap()?.flyTo(step.position, { zoom: 16 });
    },
  },
});

type Step = {
  leftColumn: string;
  timeline: string;
  timelineClasses: string[];
  description: string;
  isMovement: boolean;
  isDestination: boolean;
  position: LngLat;
  realTime: boolean;
  waitTime: number;
};

function buildSteps(itinerary: Itinerary): Step[] {
  const firstLeg = itinerary.legs[0];

  function pairwiseForEach(
    list: ItineraryLeg[],
    f: (prev: ItineraryLeg, current: ItineraryLeg | undefined) => void,
  ): void {
    for (let idx = 0; idx < list.length; idx++) {
      f(list[idx], list[idx + 1]);
    }
  }

  const originStep = {
    leftColumn: formatTime(firstLeg.startTime),
    timeline: '',
    timelineClasses: ['timeline-node', 'timeline-origin'],
    description: firstLeg.sourceName,
    isMovement: false,
    isDestination: false,
    position: firstLeg.sourceLngLat,
    realTime: firstLeg.realTime,
    waitTime: 0,
  };

  const steps = [originStep];

  pairwiseForEach(itinerary.legs, (prevLeg, currentLeg) => {
    let waitTime = 0;
    if (currentLeg) {
      waitTime = currentLeg.startTime - prevLeg.endTime;
    }
    steps.push({
      leftColumn: prevLeg.shortName,
      timeline: '',
      timelineClasses: ['timeline-edge', `timeline-edge-${prevLeg.mode}`],
      description: formatDuration(prevLeg.duration),
      position: prevLeg.sourceLngLat,
      isMovement: true,
      isDestination: false,
      realTime: prevLeg.realTime,
      waitTime: waitTime,
    });

    if (currentLeg) {
      steps.push({
        leftColumn: formatTime(currentLeg.startTime),
        timeline: '',
        timelineClasses: ['timeline-node'],
        description: currentLeg.sourceName,
        position: currentLeg.sourceLngLat,
        isMovement: false,
        isDestination: false,
        realTime: currentLeg.realTime,
        waitTime: 0,
      });
    } else {
      steps.push({
        leftColumn: formatTime(prevLeg.endTime),
        timeline: '',
        timelineClasses: ['timeline-node', 'timeline-destination'],
        description: prevLeg.destinationName,
        position: prevLeg.destinationLngLat,
        isMovement: false,
        isDestination: true,
        realTime: prevLeg.realTime,
        waitTime: 0,
      });
    }
  });

  return steps;
}
</script>

<style lang="scss">
.timeline-edge {
  position: relative;
  width: 100%;
  height: calc(100% + 20px);
  left: calc(50% - 6px);
  margin-top: -10px;
  margin-bottom: -10px;
}

.timeline-edge-WALK {
  border-left: dashed $walkColor 6px;
}

.timeline-edge:not(.timeline-edge-WALK) {
  border-left: solid $transitColor 6px;
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

.timeline-wait-time {
  opacity: 0.6;
}
</style>
