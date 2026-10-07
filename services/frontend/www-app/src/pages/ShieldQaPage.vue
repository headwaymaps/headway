<template>
  <div class="shield-qa">
    <header>
      <input v-model="filter" placeholder="Filter networks" autofocus />
      <button v-if="filter" type="button" @click="filter = ''">Clear</button>
      <span>{{ rows.length }} rows</span>
    </header>
    <div class="column-headers" :style="{ paddingLeft: `${LABEL_WIDTH}px` }">
      <span
        v-for="shieldRef in SHIELD_QA_REFS"
        :key="shieldRef"
        :style="{ width: `${COLUMN_WIDTH}px` }"
        >{{ shieldRef }}</span
      >
    </div>
    <div class="map-frame" @wheel.prevent="scroll">
      <div ref="mapContainer" class="map"></div>
      <div class="labels">
        <div
          class="label-list"
          :style="{ transform: `translateY(${-scrollTop}px)` }"
        >
          <div
            v-for="row in rows"
            :key="row"
            class="label-row"
            :style="{ height: `${ROW_HEIGHT}px` }"
          >
            <span>{{ row }}</span>
            <small v-if="adjustment(row)">{{ adjustment(row) }}</small>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Map as MaplibreMap } from 'maplibre-gl';
import type { GeoJSONSource } from 'maplibre-gl';
import { useRoute, useRouter } from 'vue-router';
import type { LocationQueryValue } from 'vue-router';
import { mapStyle } from 'src/components/BaseMap.vue';
import ShieldQa, {
  COLUMN_WIDTH,
  LABEL_WIDTH,
  ROW_HEIGHT,
  SHIELD_QA_REFS,
  SHIELD_QA_SOURCE,
  SHIELD_QA_ZOOM,
  tablePixelToLngLat,
} from 'src/models/ShieldQa';

const route = useRoute();
const router = useRouter();
const filter = ref(queryFilter(route.query.filter));
const rows = ref<string[]>([]);
const mapContainer = ref<HTMLElement>();
const labelWidth = `${LABEL_WIDTH}px`;
const SCROLL_STORAGE_KEY = 'headway.shield-qa.scroll-top';
let map: MaplibreMap | undefined;
let shieldQa: ShieldQa | undefined;
const scrollTop = ref(0);

function scroll(event: WheelEvent) {
  scrollTop.value += event.deltaY;
  positionCamera();
}

function positionCamera() {
  if (!map) {
    return;
  }
  const { clientWidth: width, clientHeight: height } = map.getContainer();
  const maxScroll = Math.max(0, rows.value.length * ROW_HEIGHT - height);
  scrollTop.value = Math.min(Math.max(scrollTop.value, 0), maxScroll);
  map.jumpTo({
    center: tablePixelToLngLat(width / 2, scrollTop.value + height / 2),
  });
  window.localStorage.setItem(SCROLL_STORAGE_KEY, String(scrollTop.value));
}

function adjustment(row: string): string {
  return shieldQa?.adjustments(row) ?? '';
}

function onResize() {
  map?.resize();
  positionCamera();
}

watch(filter, (text) => {
  void router.replace({
    query: { ...route.query, filter: text || undefined },
  });
  if (!map || !shieldQa) {
    return;
  }
  rows.value = shieldQa.rows(text);
  map
    .getSource<GeoJSONSource>(SHIELD_QA_SOURCE)
    ?.setData(shieldQa.features(rows.value));
  scrollTop.value = 0;
  positionCamera();
});

watch(
  () => route.query.filter,
  (value) => {
    const text = queryFilter(value);
    if (text !== filter.value) {
      filter.value = text;
    }
  },
);

onMounted(() => {
  const savedScrollTop = Number(
    window.localStorage.getItem(SCROLL_STORAGE_KEY),
  );
  if (Number.isFinite(savedScrollTop) && savedScrollTop >= 0) {
    scrollTop.value = savedScrollTop;
  }
  map = new MaplibreMap({
    container: mapContainer.value!,
    zoom: SHIELD_QA_ZOOM,
    center: tablePixelToLngLat(0, 0),
    interactive: false,
    attributionControl: false,
  });
  map.setStyle(mapStyle, {
    transformStyle: (_previous, next) => {
      shieldQa = ShieldQa.fromStyle(next);
      rows.value = shieldQa.rows(filter.value);
      return shieldQa.style(rows.value);
    },
  });
  map.once('style.load', positionCamera);
  window.addEventListener('resize', onResize);
});

onBeforeUnmount(() => {
  window.removeEventListener('resize', onResize);
  map?.remove();
});

function queryFilter(
  value: LocationQueryValue | LocationQueryValue[] | undefined,
): string {
  return Array.isArray(value) ? (value[0] ?? '') : (value ?? '');
}
</script>

<style scoped>
.shield-qa {
  display: flex;
  flex-direction: column;
  height: 100vh;
  font-family: sans-serif;
}

header {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 8px;
}

.column-headers {
  display: flex;
  font-size: 12px;
  color: #666;
  border-bottom: 1px solid #ccc;
}

.column-headers span {
  text-align: center;
}

.map-frame {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.map {
  position: absolute;
  inset: 0;
}

.labels {
  position: absolute;
  inset: 0 auto 0 0;
  z-index: 1;
  width: v-bind(labelWidth);
  overflow: hidden;
  background: #fff;
}

.label-list {
  will-change: transform;
}

.label-row {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 0 8px;
  user-select: text;
}

.label-row small {
  color: #777;
}
</style>
