<template>
  <div class="topLeftCard">
    <q-card>
      <search-box
        ref="searchBox"
        hint="From"
        v-model="fromPoi"
        :force-text="fromPoi ? poiDisplayName(fromPoi) : undefined"
        v-on:update:model-value="rewriteUrl"
      ></search-box>
      <search-box
        ref="searchBox"
        hint="To"
        v-model="toPoi"
        :force-text="toPoi ? poiDisplayName(toPoi) : undefined"
        v-on:update:model-value="rewriteUrl"
      ></search-box>
    </q-card>
  </div>
  <div class="bottomCard">
    <q-card>
      <q-card-section class="bg-primary text-white">
        <div class="text-subtitle1">
          {{
            from
              ? `Directions from ${poiDisplayName(fromPoi)} to ${poiDisplayName(
                  toPoi
                )}`
              : `Directions to ${poiDisplayName(toPoi)}`
          }}
        </div>
      </q-card-section>
    </q-card>
  </div>
</template>

<script lang="ts">
import { activeMarkers } from 'src/components/BaseMap.vue';
import {
  canonicalizePoi,
  decanonicalizePoi,
  POI,
  poiDisplayName,
} from 'src/components/models';
import { defineComponent, Ref, ref } from 'vue';
import SearchBox from 'src/components/SearchBox.vue';

var toPoi: Ref<POI | undefined> = ref(undefined);
var fromPoi: Ref<POI | undefined> = ref(undefined);

export default defineComponent({
  name: 'DirectionsPage',
  props: {
    mode: String,
    to: String,
    from: String,
  },
  components: { SearchBox },
  methods: {
    poiDisplayName,
    rewriteUrl: function () {
      const fromCanonical = fromPoi.value
        ? canonicalizePoi(fromPoi.value)
        : '_';
      const toCanonical = toPoi.value ? canonicalizePoi(toPoi.value) : '_';
      this.$router.replace(
        `/directions/${this.mode}/${toCanonical}/${fromCanonical}`
      );
    },
  },
  watch: {
    to: {
      immediate: true,
      deep: true,
      handler(newValue) {
        setTimeout(async () => {
          toPoi.value = await decanonicalizePoi(newValue);
        });
      },
    },
    from: {
      immediate: true,
      deep: true,
      handler(newValue) {
        setTimeout(async () => {
          fromPoi.value = await decanonicalizePoi(newValue);
        });
      },
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      toPoi.value = await decanonicalizePoi(this.$props.to as string);
      fromPoi.value = await decanonicalizePoi(this.$props.from as string);
    });
  },
  unmounted: function () {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;
  },
  setup: function () {
    return { toPoi, fromPoi };
  },
});
</script>
