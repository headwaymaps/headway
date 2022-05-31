<template>
  <div class="top-left-card">
    <q-card>
      <search-box
        ref="searchBox"
        :force-text="poiDisplayName(poi)"
        v-model="poi"
      ></search-box>
    </q-card>
  </div>

  <place-card :poi="poi" v-on:close="$router.push('/')"></place-card>
</template>

<script lang="ts">
import { Marker } from 'maplibre-gl';
import { activeMarkers, map } from 'src/components/BaseMap.vue';
import {
  canonicalizePoi,
  localizeAddress,
  POI,
  poiDisplayName,
} from 'src/components/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent } from 'vue';
import { Router } from 'vue-router';
import SearchBox from 'src/components/SearchBox.vue';

async function loadPlacePage(router: Router, osm_id_with_type: string) {
  const response = await fetch(`/nominatim/lookup/${osm_id_with_type}`);
  if (response.status != 200) {
    console.error(
      `Could not fetch POI data for ${osm_id_with_type}. Is nominatim down?`
    );
    return {};
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
  const name =
    clazz !== 'place'
      ? xmlPoi.getElementsByTagName(clazz as string).item(0)?.textContent
      : undefined;
  const suburb = xmlPoi.getElementsByTagName('suburb').item(0)?.textContent;
  const city = xmlPoi.getElementsByTagName('city').item(0)?.textContent;

  const address = localizeAddress(houseNumber, road, suburb, city);

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
  return {
    name: name,
    address: address,
    position: position,
    id: parseInt(osm_id_with_type.substring(1)),
    type: osm_id_with_type.substring(0, 1),
  };
}

export default defineComponent({
  name: 'PlacePage',
  props: {
    osm_id: String,
  },
  emits: ['loadedPoi'],
  components: { PlaceCard, SearchBox },
  data: function () {
    return {
      poi: {},
    };
  },
  watch: {
    poi(newValue) {
      setTimeout(async () => {
        if (newValue) {
          await loadPlacePage(this.$router, canonicalizePoi(newValue));
          this.$emit('loadedPoi', this.$data.poi);
        } else {
          this.$router.push('/');
        }
      });
    },
  },
  methods: {
    poiDisplayName,
    poiSelected: function (poi?: POI) {
      activeMarkers.forEach((marker) => marker.remove());
      activeMarkers.length = 0;
      if (poi?.id) {
        this.$router.push(`/place/${poi?.type}${poi?.id}`);
      } else {
        this.$router.push('/');
      }
    },
  },
  mounted: async function () {
    setTimeout(async () => {
      this.$data.poi = (await loadPlacePage(
        this.$router,
        this.$props.osm_id as string
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
      )) as any;
      this.$emit('loadedPoi', this.$data.poi);
    });
  },
  setup: function () {
    return {};
  },
});
</script>
