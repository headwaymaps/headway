<template>
  <q-list>
    <div
      v-for="item in trip.valhallaRoute.legs[0].maneuvers"
      v-bind:key="JSON.stringify(item)"
    >
      <q-item class="q-my-sm" active-class="bg-blue-1">
        <q-item-section avatar>
          <q-icon :name="valhallaTypeToIcon(item.type)" />
        </q-item-section>
        <q-item-section>
          <q-item-label>
            {{ item.instruction }}
          </q-item-label>
          <q-item-label caption>
            {{ item.verbal_post_transition_instruction }}
          </q-item-label>
        </q-item-section>
      </q-item>
      <q-separator spaced />
    </div>
  </q-list>
</template>

<script lang="ts">
import Route from 'src/models/Route';
import { defineComponent, PropType } from 'vue';
import { valhallaTypeToIcon } from 'src/services/ValhallaClient';

export default defineComponent({
  name: 'SingleModeSteps',
  props: {
    trip: {
      type: Object as PropType<Route>,
      required: true,
    },
  },
  methods: {
    valhallaTypeToIcon,
  },
});
</script>
