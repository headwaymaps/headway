<template>
  <div class="overMap">
    <q-card>
      <q-card-section class="bg-primary text-white">
        <div class="text-h6"></div>
        <div class="text-subtitle2">by John Doe</div>
      </q-card-section>
    </q-card>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';

export default defineComponent({
  name: 'PlacePage',
  props: { place_id: string },
  components: {},
});
</script>
