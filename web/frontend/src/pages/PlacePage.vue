<template>
  <div class="overMap">
    <place-card :poi="poi" v-on:close="$router.push('/')"></place-card>
  </div>
</template>

<script lang="ts">
import { Marker } from 'maplibre-gl';
import { activeMarkers, map } from 'src/components/BaseMap.vue';
import { localizeAddress, POI } from 'src/components/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent, Ref, ref } from 'vue';
import { Router } from 'vue-router';

var poi: Ref<POI | undefined> = ref(undefined);

async function loadPlacePage(router: Router, osm_id_with_type: string) {
  router.replace(`/place/${osm_id_with_type}`); // TODO: do we need to do this?

  const response = await fetch(`/nominatim/lookup/${osm_id_with_type}`);
  if (response.status != 200) {
    console.error(
      `Could not fetch POI data for ${osm_id_with_type}. Is nominatim down?`
    );
    return;
  }
  const text = await response.text();
  const parser = new DOMParser();
  const xmlPoi = parser.parseFromString(text, 'text/xml');
  const placeTag = xmlPoi.getElementsByTagName('place').item(0);
  const position = {
    lat: parseFloat(
      placeTag?.attributes?.getNamedItem('lat')?.textContent as string
    ),
    long: parseFloat(
      placeTag?.attributes?.getNamedItem('lon')?.textContent as string
    ),
  };
  const houseNumber = xmlPoi
    .getElementsByTagName('house_number')
    .item(0)?.textContent;
  const clazz = placeTag?.attributes?.getNamedItem('class')
    ?.textContent as string;
  const road = xmlPoi.getElementsByTagName('road').item(0)?.textContent;
  const name = xmlPoi
    .getElementsByTagName(clazz as string)
    .item(0)?.textContent;
  const suburb = xmlPoi.getElementsByTagName('suburb').item(0)?.textContent;
  const city = xmlPoi.getElementsByTagName('city').item(0)?.textContent;

  const address = localizeAddress(houseNumber, road, suburb, city);

  poi.value = {
    name: name,
    address: address,
    position: position,
    id: parseInt(osm_id_with_type.substring(1)),
    type: osm_id_with_type.substring(0, 1),
  };

  map?.flyTo({
    center: [position.long, position.lat],
    zoom: 16,
  });
  if (map) {
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
    osm_id: String,
  },
  emits: ['loadedPoi'],
  components: { PlaceCard },
  watch: {
    osm_id: {
      immediate: true,
      deep: true,
      handler(newValue) {
        setTimeout(async () => {
          await loadPlacePage(this.$router, newValue);
          this.$emit('loadedPoi', poi.value);
        });
      },
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      await loadPlacePage(this.$router, this.$props.osm_id as string);
      this.$emit('loadedPoi', poi.value);
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
