<template>
  <div class="overMap">
    <place-card :poi="poi" v-on:close="$router.push('/')"></place-card>
  </div>
</template>

<script lang="ts">
import { Marker } from 'maplibre-gl';
import { activeMarkers, map } from 'src/components/BaseMap.vue';
import { LongLat, POI } from 'src/components/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent, Ref, ref } from 'vue';
import { Router } from 'vue-router';

var poi: Ref<POI | undefined> = ref(undefined);

async function loadDroppedPinPage(
  router: Router,
  position: LongLat,
  fly = false
) {
  if (!map) {
    setTimeout(() => loadDroppedPinPage(router, position), 100);
    return;
  }
  poi.value = {
    name: 'Dropped Pin',
    address: undefined,
    position: position,
    id: undefined,
    type: undefined,
  };

  if (fly) {
    map?.flyTo({
      center: [position.long, position.lat],
      zoom: 12,
    });
  }
  if (map) {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;

    const marker = new Marker({ color: '#111111' }).setLngLat([
      position.long,
      position.lat,
    ]);
    marker.addTo(map);
    activeMarkers.push(marker);
  }
}

export default defineComponent({
  name: 'PlacePage',
  props: {
    long: String,
    lat: String,
  },
  components: { PlaceCard },
  watch: {
    lat: {
      immediate: true,
      deep: true,
      handler() {
        setTimeout(async () => {
          const position: LongLat = {
            long: parseFloat(this.$props.long as string),
            lat: parseFloat(this.$props.lat as string),
          };
          await loadDroppedPinPage(this.$router, position);
        });
      },
    },
    long: {
      immediate: true,
      deep: true,
      handler() {
        setTimeout(async () => {
          const position: LongLat = {
            long: parseFloat(this.$props.long as string),
            lat: parseFloat(this.$props.lat as string),
          };
          await loadDroppedPinPage(this.$router, position);
        });
      },
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      const position: LongLat = {
        long: parseFloat(this.$props.long as string),
        lat: parseFloat(this.$props.lat as string),
      };
      await loadDroppedPinPage(this.$router, position, true);
    });
  },
  unmounted: function () {
    activeMarkers.forEach((marker) => marker.remove());
    activeMarkers.length = 0;
  },
  setup: function () {
    return { poi };
  },
});
</script>
