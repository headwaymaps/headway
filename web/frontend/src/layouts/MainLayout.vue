<template>
  <div class="app-container" :class="appClass">
    <router-view />
    <base-map />
  </div>
</template>

<style lang="scss">
.app-container {
  width: 100vw;
  height: 100vh;
}

.app-container {
  width: 100%;
  // iPhoneX content is beyond screen boundary
  //height: calc(100vh - 50px);
  height: 100vh;
  display: flex;
  flex-direction: column;
  flex-wrap: wrap-reverse;
}

.top-card {
  border-bottom: solid #ccc 1px;
  @media screen and (max-width: 800px) {
    order: 1;

    box-shadow: 0px 0px 5px #00000088;
    // need z-index to cast shadow onto map
    z-index: 1;
  }
  @media screen and (min-width: 800px) {
    order: 1;
    width: max(33%, 370px);
  }
  padding: 16px;
}

.bottom-card {
  overflow-y: scroll;
  @media screen and (max-width: 800px) {
    order: 3;
    width: 100%;
    box-shadow: 0px 0px 5px #00000088;
    // need z-index to cast shadow onto map
    z-index: 1;
  }

  @media screen and (min-width: 800px) {
    order: 2;
    width: max(33%, 370px);
    flex: 1;
  }
}

#map {
  @media screen and (max-width: 800px) {
    // This is tall enough to keep the map UI from overlapping.
    // Ironically the "wide"/"desktop" layout is slightly less tall than the
    // "mobile optimized" layout, which only needs about 170px
    min-height: 190px;
    width: 100%;
    order: 2;
    flex: 1;
  }

  @media screen and (min-width: 800px) {
    width: min(67%, calc(100% - 370px));
    height: 100%;
  }
}

// Interface casts shadows on map
#map:before {
  // Left inner shadow
  @media screen and (min-width: 800px) {
    content: '';
    position: absolute;
    pointer-events: none;
    // need z-index to cast shadow onto map
    z-index: 1;
    height: 100%;
    width: 10px;
    box-shadow: inset 5px 0px 5px -5px #00000088;
  }
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';
import BaseMap from 'src/components/BaseMap.vue';

export default defineComponent({
  name: 'MainLayout',
  props: {
    appClass: String,
  },
  components: { BaseMap },
});
</script>
