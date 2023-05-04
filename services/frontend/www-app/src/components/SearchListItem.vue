<template>
  <q-item class="list-item" active-class="list-item--selected" :active="active">
    <q-item-section>
      <q-item-label>
        {{ place.name }}
      </q-item-label>
      <q-item-label class="text-weight-light">
        {{ place.address }}
      </q-item-label>
    </q-item-section>
  </q-item>
</template>

<script lang="ts">
import Place from 'src/models/Place';
import { defineComponent, PropType } from 'vue';

export default defineComponent({
  name: 'SearchListItem',
  props: {
    place: {
      type: Object as PropType<Place>,
      required: true,
    },
    active: Boolean,
  },
});
</script>
