<template>
  <teleport to="#map .maplibregl-ctrl-bottom-left">
    <div class="headway-map-debug-panel maplibregl-ctrl">
      <label v-if="showToggle" class="headway-map-debug-panel__title">
        <input v-model="debugEnabled" type="checkbox" />
        Debug
      </label>
      <div v-else class="headway-map-debug-panel__title">Debug</div>
      <div class="headway-map-debug-panel__row">
        <span>zoom</span>
        <span class="headway-map-debug-panel__value">{{
          zoom.toFixed(2)
        }}</span>
      </div>
      <label class="headway-map-debug-panel__option">
        <input v-model="showRoadTags" type="checkbox" />
        road tags
      </label>
      <label class="headway-map-debug-panel__option">
        <input v-model="showBuildingTags" type="checkbox" />
        building tags
      </label>
    </div>
  </teleport>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Popup } from 'maplibre-gl';
import type { Map as MaplibreMap, MapMouseEvent, Point } from 'maplibre-gl';
import { debugEnabled } from 'src/utils/debug';

const props = defineProps<{ map: unknown }>();
const map = props.map as MaplibreMap;
const showToggle = process.env.DEV;

const showRoadTagsKey = 'debug.showRoadTags';
const showBuildingTagsKey = 'debug.showBuildingTags';
const oldShowRoadDesignationKey = 'debug.showRoadDesignation';
const storedRoadTags =
  window.localStorage.getItem(showRoadTagsKey) ??
  window.localStorage.getItem(oldShowRoadDesignationKey);
const showRoadTags = ref(storedRoadTags === 'true');
const showBuildingTags = ref(
  window.localStorage.getItem(showBuildingTagsKey) === 'true',
);
const zoom = ref(map.getZoom());
const popup = new Popup({
  closeButton: false,
  closeOnClick: false,
  maxWidth: '720px',
  offset: 12,
});
/// A clicked popup stays put until the next click or Escape, so its sections can be opened.
let pinned = false;

watch(showRoadTags, (enabled) => {
  window.localStorage.setItem(showRoadTagsKey, String(enabled));
});

watch(showBuildingTags, (enabled) => {
  window.localStorage.setItem(showBuildingTagsKey, String(enabled));
});

watch([showRoadTags, showBuildingTags], closePopup);

function closePopup() {
  pinned = false;
  popup.remove();
}

function tagsTable(title: string, tags: Record<string, unknown>): HTMLElement {
  const section = document.createElement('div');
  const heading = document.createElement('div');
  heading.className = 'headway-debug-popup__heading';
  heading.textContent = title;
  const table = document.createElement('table');
  for (const [key, value] of Object.entries(tags).sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    const row = table.insertRow();
    row.insertCell().textContent = key;
    row.insertCell().textContent = String(value);
  }
  section.append(heading, table);
  return section;
}

/// The tags of what's at `point` on the left, and the page's own details on the right.
function popupContent(point: Point): HTMLElement | undefined {
  const tags = document.createElement('div');
  const features = map.queryRenderedFeatures(point);
  if (showRoadTags.value) {
    const road = features.find(
      (feature) =>
        feature.sourceLayer === 'transportation' &&
        typeof feature.properties?.class === 'string',
    );
    if (road?.properties) {
      const name = roadName(road);
      tags.append(
        tagsTable('Road', { ...road.properties, ...(name ? { name } : {}) }),
      );
    }
  }
  if (showBuildingTags.value) {
    const building = features.find(
      (feature) => feature.sourceLayer === 'building',
    );
    if (building?.properties) {
      tags.append(tagsTable('Building', building.properties));
    }
  }
  if (!tags.hasChildNodes()) {
    return undefined;
  }

  const root = document.createElement('div');
  root.className = 'headway-debug-popup';
  const hint = document.createElement('div');
  hint.className = 'headway-debug-popup__hint';
  hint.textContent = pinned
    ? 'pinned · click the map or Esc to close'
    : 'click to pin';
  root.append(tags, hint);
  return root;
}

function showPopup(event: MapMouseEvent): boolean {
  const content = popupContent(event.point);
  if (!content) {
    popup.remove();
    return false;
  }
  popup.setLngLat(event.lngLat).setDOMContent(content).addTo(map);
  return true;
}

function roadName(road: { properties?: Record<string, unknown> }): unknown {
  if (road.properties?.name) {
    return road.properties.name;
  }

  // Names live in OpenMapTiles' separate transportation_name layer. Match its
  // feature to the road geometry when the transportation feature has no name.
  const osmId = road.properties?.osm_id;
  if (osmId === undefined) {
    return undefined;
  }
  return map
    .querySourceFeatures('openmaptiles', { sourceLayer: 'transportation_name' })
    .find((feature) => feature.properties?.osm_id === osmId)?.properties?.name;
}

function onMapMouseMove(event: MapMouseEvent) {
  if (!pinned) showPopup(event);
}

/// Pins the popup on what was clicked, or dismisses a pinned one.
function onMapClick(event: MapMouseEvent) {
  pinned = !pinned;
  if (pinned && !showPopup(event)) pinned = false;
  if (!pinned) closePopup();
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') closePopup();
}

function onMove() {
  zoom.value = map.getZoom();
}

function onCanvasMouseLeave() {
  if (!pinned) popup.remove();
}

onMounted(() => {
  map.on('move', onMove);
  map.on('mousemove', onMapMouseMove);
  map.on('click', onMapClick);
  map.getCanvas().addEventListener('mouseleave', onCanvasMouseLeave);
  window.addEventListener('keydown', onKey);
});

onBeforeUnmount(() => {
  map.off('move', onMove);
  map.off('mousemove', onMapMouseMove);
  map.off('click', onMapClick);
  map.getCanvas().removeEventListener('mouseleave', onCanvasMouseLeave);
  window.removeEventListener('keydown', onKey);
  popup.remove();
});
</script>

<style lang="scss" scoped>
.headway-map-debug-panel {
  padding: 6px 8px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.3);
  font-family: monospace;
  font-size: 12px;
  line-height: 1.4;
  color: black;

  &__title {
    display: flex;
    align-items: center;
    gap: 4px;
    font-weight: 600;
  }

  &__row {
    display: flex;
    gap: 8px;
    justify-content: space-between;
  }

  &__value {
    font-variant-numeric: tabular-nums;
  }

  &__option {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
  }
}
</style>

<style lang="scss">
.headway-debug-popup {
  font-size: 12px;
  color: black;
  line-height: 1.4;

  &__heading {
    font-weight: 600;
  }

  &__hint {
    color: #777;
    font-size: 11px;
  }

  table {
    border-collapse: collapse;
    margin: 2px 0 4px;
  }

  td {
    padding: 1px 6px;
    vertical-align: top;
  }

  td:first-child {
    color: #555;
    white-space: nowrap;
  }
}
</style>
