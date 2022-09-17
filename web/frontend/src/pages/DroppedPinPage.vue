<template>
  <div class="top-left-card">
    <q-card>
      <q-card-section>
        <search-box
          ref="searchBox"
          v-on:poi_selected="poiSelected"
          v-on:poi_hovered="poiHovered"
        ></search-box>
      </q-card-section>
    </q-card>
  </div>

  <div class="bottom-card">
    <place-card :poi="poi" v-on:close="$router.push('/')"></place-card>
  </div>
</template>

<script lang="ts">
import { Marker } from 'maplibre-gl';
import { getBaseMap, map } from 'src/components/BaseMap.vue';
import { POI } from 'src/utils/models';
import PlaceCard from 'src/components/PlaceCard.vue';
import { defineComponent, Ref, ref } from 'vue';
import { Router } from 'vue-router';
import SearchBox from 'src/components/SearchBox.vue';
import { LongLat } from 'src/utils/geomath';
import { i18n } from 'src/i18n/lang';

var poi: Ref<POI | undefined> = ref(undefined);

async function loadDroppedPinPage(router: Router, position: LongLat) {
  if (!map) {
    setTimeout(() => loadDroppedPinPage(router, position), 100);
    return;
  }
  poi.value = {
    name: i18n.global.t('dropped_pin'),
    address: undefined,
    position: position,
  };

  getBaseMap()?.flyTo([position.long, position.lat], 16);
  getBaseMap()?.pushMarker(
    'active_marker',
    new Marker({ color: '#111111' }).setLngLat([position.long, position.lat])
  );
  getBaseMap()?.removeMarkersExcept(['active_marker']);
}

export default defineComponent({
  name: 'DroppedPinPage',
  props: {
    long: String,
    lat: String,
  },
  components: { PlaceCard, SearchBox },
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
  methods: {
    poiSelected: function (poi?: POI) {
      getBaseMap()?.removeMarkersExcept(['active_marker']);
      if (poi?.gid) {
        const gidComponent = encodeURIComponent(poi?.gid);
        this.$router.push(`/place/${gidComponent}`);
      } else {
        this.$router.push('/');
      }
    },
    poiHovered: function (poi?: POI) {
      if (poi?.position) {
        getBaseMap()?.pushMarker(
          'hover_marker',
          new Marker({ color: '#11111155' }).setLngLat([
            poi.position.long,
            poi.position.lat,
          ])
        );
        getBaseMap()?.removeMarkersExcept(['hover_marker']);
      }
    },
  },

  mounted: async function () {
    setTimeout(async () => {
      const position: LongLat = {
        long: parseFloat(this.$props.long as string),
        lat: parseFloat(this.$props.lat as string),
      };
      await loadDroppedPinPage(this.$router, position);
    });
  },
  setup: function () {
    return { poi };
  },
});
</script>
