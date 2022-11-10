<template>
  <q-layout view="lHh Lpr lFf">
    <div class="mainContainer">
      <router-view />
      <base-map ref="basemap" />
    </div>
  </q-layout>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import BaseMap from 'src/components/BaseMap.vue';

export default defineComponent({
  name: 'MainLayout',

  components: { BaseMap },
  props: {
    osm_id: String,
  },
  methods: {},

  setup() {
    const leftDrawerOpen = ref(false);

    return {
      leftDrawerOpen,
      toggleLeftDrawer() {
        leftDrawerOpen.value = !leftDrawerOpen.value;
      },
    };
  },
});
</script>
