<template>
  <teleport to="#map">
    <div class="headway-map-debug-panel">
      <div class="headway-map-debug-panel__title">Debug</div>
      <div class="headway-map-debug-panel__row">
        <span>zoom</span>
        <span class="headway-map-debug-panel__value">{{
          zoom.toFixed(2)
        }}</span>
      </div>
      <label class="headway-map-debug-panel__option">
        <input v-model="showRoadTags" type="checkbox" />
        show road tags
      </label>
      <label class="headway-map-debug-panel__option">
        <input v-model="showBuildingTags" type="checkbox" />
        show building tags
      </label>
    </div>
  </teleport>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { Popup } from 'maplibre-gl';
import type { Map as MaplibreMap, MapMouseEvent } from 'maplibre-gl';

const props = defineProps<{ map: unknown }>();
const map = props.map as MaplibreMap;

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
const roadTagsPopup = new Popup({
  closeButton: false,
  closeOnClick: false,
  offset: 12,
});
const buildingTagsPopup = new Popup({
  closeButton: false,
  closeOnClick: false,
  offset: 12,
});

watch(showRoadTags, (enabled) => {
  window.localStorage.setItem(showRoadTagsKey, String(enabled));
  if (!enabled) {
    roadTagsPopup.remove();
  }
});

watch(showBuildingTags, (enabled) => {
  window.localStorage.setItem(showBuildingTagsKey, String(enabled));
  if (!enabled) {
    buildingTagsPopup.remove();
  }
});

function showTags(
  popup: Popup,
  event: MapMouseEvent,
  tags: Record<string, unknown>,
) {
  const tagList = document.createElement('pre');
  tagList.textContent = Object.entries(tags)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([key, value]) => `${key}=${value}`)
    .join('\n');
  popup.setLngLat(event.lngLat).setDOMContent(tagList).addTo(map);
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
  const features = map.queryRenderedFeatures(event.point);
  if (showRoadTags.value) {
    const road = features.find(
      (feature) =>
        feature.sourceLayer === 'transportation' &&
        typeof feature.properties?.class === 'string',
    );
    if (road?.properties) {
      const name = roadName(road);
      showTags(roadTagsPopup, event, {
        ...road.properties,
        ...(name ? { name } : {}),
      });
    } else {
      roadTagsPopup.remove();
    }
  }

  if (showBuildingTags.value) {
    const building = features.find(
      (feature) => feature.sourceLayer === 'building',
    );
    if (building?.properties) {
      showTags(buildingTagsPopup, event, building.properties);
    } else {
      buildingTagsPopup.remove();
    }
  }
}

function onMove() {
  zoom.value = map.getZoom();
}

function onCanvasMouseLeave() {
  roadTagsPopup.remove();
  buildingTagsPopup.remove();
}

onMounted(() => {
  map.on('move', onMove);
  map.on('mousemove', onMapMouseMove);
  map.getCanvas().addEventListener('mouseleave', onCanvasMouseLeave);
});

onBeforeUnmount(() => {
  map.off('move', onMove);
  map.off('mousemove', onMapMouseMove);
  map.getCanvas().removeEventListener('mouseleave', onCanvasMouseLeave);
  roadTagsPopup.remove();
  buildingTagsPopup.remove();
});
</script>

<style lang="scss" scoped>
.headway-map-debug-panel {
  position: absolute;
  z-index: 2;
  bottom: 8px;
  left: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.3);
  font-family: monospace;
  font-size: 12px;
  line-height: 1.4;
  color: black;

  &__title {
    font-weight: 600;
    opacity: 0.6;
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
