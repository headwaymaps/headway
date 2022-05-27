<template>
  <div id="map"></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import maplibregl, {
  LngLatBoundsLike,
  MapMouseEvent,
  Marker,
  Popup,
} from 'maplibre-gl';
import { LongLat } from './models';
import 'maplibre-gl/dist/maplibre-gl.css';

export var map: maplibregl.Map | null = null;

export var activeMarkers: Marker[] = [];

async function loadMap() {
  const response = await fetch('/bbox.txt');
  if (response.status != 200) {
    // TODO surface error
    return;
  }
  const bbox_strings = (await response.text()).split(' ');
  const bounds = [
    parseFloat(bbox_strings[0]),
    parseFloat(bbox_strings[1]),
    parseFloat(bbox_strings[2]),
    parseFloat(bbox_strings[3]),
  ];
  const center = [(bounds[2] + bounds[0]) / 2, (bounds[3] + bounds[1]) / 2];
  const scaleFactor = 1.0 / Math.cos((3.14159 / 180) * center[1]);
  const extents = [bounds[2] - bounds[0], bounds[3] - bounds[1]];
  const maxExtent = Math.max(...extents) * scaleFactor;
  const maxBounds: LngLatBoundsLike = [
    center[0] - maxExtent,
    center[1] - maxExtent,
    center[0] + maxExtent,
    center[1] + maxExtent,
  ];
  map = new maplibregl.Map({
    container: 'map', // container id
    style: '/styles/basic-preview/style.json', // style URL
    center: [0, 0], // starting position [lng, lat]
    maxBounds: maxBounds,
    zoom: 1, // starting zoom
  });
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mapTouchTimeouts: any[] = [];

type BaseMapEventType = 'click' | 'longpress';
type BaseMapEventHandler = (event: MapMouseEvent) => void;
type BaseMapEventHandlerHandle = number;

const mapTouchHandlers: Map<
  BaseMapEventType,
  Map<number, BaseMapEventHandler>
> = new Map();
var eventHandlerCount = 0;

export function addMapHandler(
  event: BaseMapEventType,
  handler: BaseMapEventHandler
): BaseMapEventHandlerHandle {
  if (!mapTouchHandlers.get(event)) {
    mapTouchHandlers.set(event, new Map());
  }
  eventHandlerCount++;
  mapTouchHandlers.get(event)?.set(eventHandlerCount, handler);
  return eventHandlerCount;
}

export function removeMapHandler(event: BaseMapEventType, handle: number) {
  mapTouchHandlers.get(event)?.delete(handle);
}

function clearAllTimeouts() {
  mapTouchTimeouts.forEach((timeout) => clearTimeout(timeout));
  mapTouchTimeouts.length = 0;
}

export default defineComponent({
  name: 'BaseMap',
  emits: ['onMapClick', 'onMapLongPress', 'load'],
  methods: {
    flyTo(location: LongLat, zoom: number) {
      map?.flyTo({ center: [location.long, location.lat], zoom: zoom });
    },
    addToMap(item: Marker | Popup) {
      if (map) {
        item.addTo(map);
      }
      return item;
    },
  },
  mounted: async function () {
    await loadMap();
    map?.on('click', (event: MapMouseEvent) => {
      mapTouchHandlers.get('click')?.forEach((value) => value(event));
    });
    map?.on('mousedown', (event: MapMouseEvent) => {
      clearAllTimeouts();
      mapTouchTimeouts.push(
        setTimeout(() => {
          mapTouchHandlers.get('longpress')?.forEach((value) => value(event));
        }, 700)
      );
    });
    map?.on('mouseup', () => clearAllTimeouts());
    map?.on('mousemove', () => clearAllTimeouts());
    map?.on('move', () => clearAllTimeouts());
    this.$emit('load');
  },
});
</script>
