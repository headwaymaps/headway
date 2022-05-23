<template>
  <q-page class="row items-center justify-evenly">
    <base-map
    ></base-map>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import BaseMap from 'src/components/BaseMap.vue';

export default defineComponent({
  name: 'IndexPage',
  components: { BaseMap },
});
</script>
