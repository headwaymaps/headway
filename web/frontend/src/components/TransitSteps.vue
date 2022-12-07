<template>
  <q-list>
    <q-item
      v-for="step in steps"
      v-bind:key="JSON.stringify(step)"
      class="itinerary-row"
      :style="step.isDestination ? 'padding-bottom: 20px;' : ''"
      :clickable="!step.isMovement"
      v-on:click="clickedStep(step)"
    >
      <q-item-section
        :top="step.isMovement"
        class="col-3 timeline-time-mode"
        :class="step.isDestination ? 'timeline-destination' : ''"
        style="text-align: right; padding-right: 6px"
        >{{ step.time }}</q-item-section
      >
      <q-item-section class="col-1">
        <div
          :class="step.timelineClasses"
          style="text-align: center"
          v-if="!step.isDestination"
        >
          {{ step.timeline }}
        </div>
        <div
          :class="step.timelineClasses"
          style="text-align: center"
          v-if="step.isDestination"
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
            step.isDestination ? 'timeline-destination' : ''
          )
        "
        >{{ step.description }}</q-item-section
      >
    </q-item>
  </q-list>
</template>

<style lang="scss">
.itinerary-row {
  padding: 0;
}

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

.timeline-edge-BUS {
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
</style>

<script lang="ts">
import Itinerary from 'src/models/Itinerary';
import { defineComponent, PropType } from 'vue';
import { formatDuration, formatTime } from 'src/utils/format';
import { LngLat } from 'maplibre-gl';
import { getBaseMap } from 'src/components/BaseMap.vue';

export default defineComponent({
  name: 'TransitSteps',
  data: function () {
    return {
      steps: buildSteps(this.$props.itinerary),
    };
  },
  props: {
    itinerary: {
      type: Object as PropType<Itinerary>,
      required: true,
    },
  },
  methods: {
    formatTime,
    formatDuration,
    clickedStep: (step): void => {
      getBaseMap()?.flyTo([step.position.lng, step.position.lat], 16);
    },
  },
});

type Step = {
  time: string;
  timeline: string;
  timelineClasses: string[];
  description: string;
  isMovement: boolean;
  isDestination: boolean;
  position: LngLat;
};

function buildSteps(itinerary: Itinerary): Step[] {
  const firstLeg = itinerary.legs[0];

  const originStep = {
    time: formatTime(firstLeg.startTime),
    timeline: '',
    timelineClasses: ['timeline-node', 'timeline-origin'],
    description: firstLeg.sourceName,
    isMovement: false,
    isDestination: false,
    position: firstLeg.sourceLngLat,
  };

  let middleSteps = itinerary.legs.flatMap((leg) => {
    return [
      {
        time: leg.shortName,
        timeline: '',
        timelineClasses: ['timeline-edge', `timeline-edge-${leg.mode}`],
        description: formatDuration(leg.duration),
        position: leg.sourceLngLat,
        isMovement: true,
        isDestination: false,
      },
      {
        time: formatTime(leg.endTime),
        timeline: '',
        timelineClasses: ['timeline-node'],
        description: leg.destinationName,
        position: leg.destinationLngLat,
        isMovement: false,
        isDestination: false,
      },
    ];
  });

  const destinationStep = middleSteps[middleSteps.length - 1];
  destinationStep.timelineClasses.push('timeline-destination');
  destinationStep.isDestination = true;

  return [originStep].concat(middleSteps);
}
</script>
