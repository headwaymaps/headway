<template>
  <div class="bottom-card building-light-panel">
    <template v-if="ready">
      <label class="building-light-panel__before">
        <input v-model="before" type="checkbox" />
        before (shipped lighting)
      </label>

      <fieldset :disabled="before">
        <legend>light</legend>
        <label>
          intensity
          <input
            v-model.number="tuned.intensity"
            type="range"
            min="0"
            max="1"
            step="0.01"
          />
          <span>{{ tuned.intensity.toFixed(2) }}</span>
        </label>
        <label>
          anchor
          <select v-model="tuned.anchor">
            <option value="viewport">viewport</option>
            <option value="map">map</option>
          </select>
        </label>
        <label>
          distance
          <input
            v-model.number="tuned.radial"
            type="range"
            min="0.5"
            max="3"
            step="0.05"
          />
          <span>{{ tuned.radial.toFixed(2) }}</span>
        </label>
        <label>
          direction
          <input
            v-model.number="tuned.azimuthal"
            type="range"
            min="0"
            max="360"
            step="1"
          />
          <span>{{ tuned.azimuthal }}°</span>
        </label>
        <label>
          overhead
          <input
            v-model.number="tuned.polar"
            type="range"
            min="0"
            max="90"
            step="1"
          />
          <span>{{ tuned.polar }}°</span>
        </label>
        <label>
          color
          <input v-model="tuned.color" type="color" />
        </label>
      </fieldset>

      <fieldset :disabled="before">
        <legend>3d buildings</legend>
        <label>
          opacity
          <input
            v-model.number="tuned.opacity"
            type="range"
            min="0"
            max="1"
            step="0.01"
          />
          <span>{{ tuned.opacity.toFixed(2) }}</span>
        </label>
        <label>
          height
          <input
            v-model.number="tuned.heightScale"
            type="range"
            min="0"
            max="1"
            step="0.01"
          />
          <span>{{ tuned.heightScale.toFixed(2) }}</span>
        </label>
        <label>
          corner distance
          <input
            v-model.number="tuned.cornerDistance"
            type="range"
            min="0"
            max="8"
            step="0.25"
          />
          <span>{{ tuned.cornerDistance.toFixed(2) }} m</span>
        </label>
        <label>
          roof color
          <input v-model="tuned.roofColor" type="color" />
        </label>
        <label>
          <input v-model="tuned.verticalGradient" type="checkbox" />
          darken wall bases
        </label>
      </fieldset>

      <button
        type="button"
        :disabled="before"
        @click="useSettings(shippedSettings)"
      >
        reset
      </button>
      <pre>{{ summary }}</pre>
    </template>
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from 'vue';
import type { AllPaintProperties } from '@maplibre/maplibre-gl-style-spec';
import type {
  ExpressionSpecification,
  LightSpecification,
  Map as MaplibreMap,
} from 'maplibre-gl';
import { baseMapPromise, map as baseMap } from 'src/components/BaseMap.vue';

const layerId = 'building_3d';
const paintProperties = [
  'fill-extrusion-color',
  'fill-extrusion-opacity',
  'fill-extrusion-height',
  'fill-extrusion-base',
  'fill-extrusion-vertical-gradient',
] as const;
type PaintProperty = (typeof paintProperties)[number];
type Paint = { [K in PaintProperty]: AllPaintProperties[K] };
const storageKey = 'dev.buildingLight';

/** What the style ships with, captured when the page opens. */
type Shipped = {
  light: LightSpecification;
  paint: Paint;
  cornerDistance: number;
};

type Settings = {
  intensity: number;
  anchor: 'map' | 'viewport';
  radial: number;
  azimuthal: number;
  polar: number;
  color: string;
  opacity: number;
  heightScale: number;
  verticalGradient: boolean;
  cornerDistance: number;
  roofColor: string;
};

// What basic-v4 ships: MapLibre's default light, dimmed, and translucent buildings.
const shippedSettings: Settings = {
  intensity: 0.3,
  anchor: 'viewport',
  radial: 1.15,
  azimuthal: 210,
  polar: 30,
  color: '#ffffff',
  opacity: 0.8,
  heightScale: 0.8,
  verticalGradient: false,
  cornerDistance: 3,
  roofColor: '#d7decc',
};

function storedSettings(): Settings {
  try {
    const stored = window.localStorage.getItem(storageKey);
    return stored
      ? { ...shippedSettings, ...JSON.parse(stored) }
      : { ...shippedSettings };
  } catch {
    return { ...shippedSettings };
  }
}

const ready = ref(false);
const before = ref(false);
const tuned = reactive<Settings>(storedSettings());
let map: MaplibreMap;
let shipped: Shipped;
let fadeInZoom: number;

function extrude(property: string, scale: number): ExpressionSpecification {
  return ['*', ['get', property], scale];
}

function tunedLight(): LightSpecification {
  return {
    anchor: tuned.anchor,
    color: tuned.color,
    intensity: tuned.intensity,
    position: [tuned.radial, tuned.azimuthal, tuned.polar],
  };
}

function tunedPaint(): Paint {
  return {
    'fill-extrusion-color': tuned.roofColor,
    'fill-extrusion-opacity': [
      'interpolate',
      ['linear'],
      ['zoom'],
      fadeInZoom,
      0,
      fadeInZoom + 0.5,
      tuned.opacity,
    ],
    'fill-extrusion-height': extrude('render_height', tuned.heightScale),
    'fill-extrusion-base': extrude('render_min_height', tuned.heightScale),
    'fill-extrusion-vertical-gradient': tuned.verticalGradient,
  };
}

function apply() {
  const light = before.value ? shipped.light : tunedLight();
  const paint = before.value ? shipped.paint : tunedPaint();
  map.setLight(light);
  const set = <K extends PaintProperty>(property: K) =>
    map.setPaintProperty(layerId, property, paint[property]);
  paintProperties.forEach(set);
  map.setLayoutProperty(
    layerId,
    'fill-extrusion-rounded-corner-distance',
    before.value ? shipped.cornerDistance : tuned.cornerDistance,
  );
}

function useSettings(settings: Settings) {
  Object.assign(tuned, settings);
}

const summary = computed(() =>
  JSON.stringify(
    {
      light: tunedLight(),
      [layerId]: {
        color: tuned.roofColor,
        opacity: tuned.opacity,
        heightScale: tuned.heightScale,
        verticalGradient: tuned.verticalGradient,
        cornerDistance: tuned.cornerDistance,
      },
    },
    null,
    1,
  ),
);

watch([before, tuned], () => {
  try {
    window.localStorage.setItem(storageKey, JSON.stringify(tuned));
  } catch {
    // Settings just won't persist.
  }
  apply();
});

onMounted(async () => {
  await baseMapPromise;
  if (!baseMap) {
    throw new Error('base map must be loaded');
  }
  map = baseMap;
  (window as unknown as { buildingDebugMap: MaplibreMap }).buildingDebugMap =
    map;
  if (!map.getLayer(layerId)) {
    throw new Error(`style must have a ${layerId} layer (basic-v4 or later)`);
  }
  fadeInZoom = map.getLayer(layerId)!.minzoom;
  shipped = {
    light: map.getLight() ?? {},
    paint: Object.fromEntries(
      paintProperties.map((property) => [
        property,
        map.getPaintProperty(layerId, property),
      ]),
    ) as Paint,
    cornerDistance: map.getLayoutProperty(
      layerId,
      'fill-extrusion-rounded-corner-distance',
    ) as number,
  };
  apply();
  ready.value = true;
});

onBeforeUnmount(() => {
  if (!shipped) return;
  before.value = true;
  apply();
});
</script>

<style lang="scss" scoped>
.building-light-panel {
  padding: 8px;
  font-family: monospace;
  font-size: 12px;
  color: black;

  fieldset {
    margin: 6px 0;
    border: 1px solid rgba(0, 0, 0, 0.15);
  }

  label {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 3px 0;
  }

  input[type='range'] {
    flex: 1;
    min-width: 0;
  }

  &__before {
    font-weight: 600;
    font-size: 14px;
  }

  pre {
    margin: 6px 0 0;
    font-size: 10px;
    white-space: pre-wrap;
  }
}
</style>
