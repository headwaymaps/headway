<template>
  <div id="map"></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import maplibregl, {
  LngLatBoundsLike,
  MapMouseEvent,
  Marker,
} from 'maplibre-gl';
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
  let bounds = undefined;
  if (bbox_strings.length !== 4) {
    map = new maplibregl.Map({
      container: 'map', // container id
      style: '/styles/style/style.json', // style URL
      zoom: 1, // starting zoom
      trackResize: true,
    });
  } else {
    bounds = [
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
      style: '/styles/style/style.json', // style URL
      center: [0, 0], // starting position [lng, lat]
      maxBounds: maxBounds,
      zoom: 1, // starting zoom
      trackResize: true,
    });
  }
}

const mapTouchTimeouts: NodeJS.Timeout[] = [];

type BaseMapEventType = 'click' | 'longpress' | 'poi_click';
type BaseMapEventHandler = (
  event: MapMouseEvent & {
    features?: GeoJSON.Feature[] | undefined;
  }
) => void;
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

var bottomCardAllowance = 0;

export function setBottomCardAllowance(pixels?: number) {
  if (pixels !== undefined) {
    bottomCardAllowance = pixels;
  }
  const mapElement = document.getElementsByClassName(
    'maplibregl-map'
  )[0] as HTMLDivElement;
  const topLeftCard = document.getElementsByClassName(
    'top-left-card'
  )[0] as HTMLDivElement;
  var topLeftCardAdjustment = 0;
  console.log(topLeftCard);
  console.log(topLeftCard.style.position);
  if (
    topLeftCard &&
    window.getComputedStyle(topLeftCard).position !== 'fixed'
  ) {
    topLeftCardAdjustment = topLeftCard.offsetHeight;
  }
  if (map !== null) {
    mapElement.style.height = `${
      window.innerHeight - bottomCardAllowance - topLeftCardAdjustment
    }px`;
  }
  map?.resize();
}

var baseMapMethods:
  | {
      flyTo: (location: [number, number], zoom: number) => void;
      fitBounds: (bounds: LngLatBoundsLike) => void;
    }
  | undefined = undefined;

// There really has to be a better way to do this, but we only ever have 1 base map so I guess it works.
export function getBaseMap() {
  return baseMapMethods;
}

export default defineComponent({
  name: 'BaseMap',
  data: function (): {
    flyToLocation: { center: [number, number]; zoom: number } | undefined;
    boundsToFit: LngLatBoundsLike | undefined;
    hasGeolocated: boolean;
  } {
    return {
      flyToLocation: undefined,
      boundsToFit: undefined,
      hasGeolocated: false,
    };
  },
  methods: {
    flyTo: async function (location: [number, number], zoom: number) {
      const permissionResult = await navigator.permissions.query({
        name: 'geolocation',
      });
      if (
        this.$data.hasGeolocated === true ||
        permissionResult.state !== 'granted'
      ) {
        map?.flyTo({ center: location, zoom: zoom }, { flying: true });
      } else {
        this.$data.flyToLocation = { center: location, zoom: zoom };
      }
    },
    fitBounds: async function (bounds: LngLatBoundsLike) {
      const permissionResult = await navigator.permissions.query({
        name: 'geolocation',
      });
      if (
        this.$data.hasGeolocated === true ||
        permissionResult.state !== 'granted'
      ) {
        map?.fitBounds(bounds, {
          padding: Math.min(window.innerWidth, window.innerHeight) / 8,
        });
      } else {
        this.$data.boundsToFit = bounds;
      }
    },
  },
  mounted: async function () {
    await loadMap();
    // This might be the ugliest thing in this whole web app. Expose methods through an internal thing.
    baseMapMethods = { flyTo: this.flyTo, fitBounds: this.fitBounds };
    var nav = new maplibregl.NavigationControl({
      visualizePitch: true,
      showCompass: true,
      showZoom: true,
    });
    map?.addControl(nav, 'top-right');
    var geolocate = new maplibregl.GeolocateControl({
      positionOptions: { enableHighAccuracy: true },
      showAccuracyCircle: true,
      showUserLocation: true,
    });
    map?.addControl(geolocate, 'bottom-right');
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
    map?.on('touchstart', (event: MapMouseEvent) => {
      clearAllTimeouts();
      mapTouchTimeouts.push(
        setTimeout(() => {
          mapTouchHandlers.get('longpress')?.forEach((value) => value(event));
        }, 700)
      );
    });
    map?.on('mouseup', () => clearAllTimeouts());
    map?.on('mousemove', () => clearAllTimeouts());
    map?.on('touchup', () => clearAllTimeouts());
    map?.on('touchend', () => clearAllTimeouts());
    map?.on('move', () => clearAllTimeouts());
    setTimeout(async () => {
      const result = await navigator.permissions.query({ name: 'geolocation' });
      if (result.state === 'granted') {
        map?.on('load', () => {
          geolocate.trigger();
          geolocate.on('geolocate', () => {
            if (!this.$data.hasGeolocated) {
              map?.stop();
              if (this.$data.flyToLocation) {
                map?.flyTo(this.$data.flyToLocation, { flying: true });
                this.$data.flyToLocation = undefined;
              } else if (this.$data.boundsToFit) {
                this.fitBounds(this.$data.boundsToFit);
                this.$data.boundsToFit = undefined;
              }
            }
            this.$data.hasGeolocated = true;
          });
        });
      }
    });
    map?.on('load', () => {
      setBottomCardAllowance();
      const layers = map?.getStyle().layers;
      if (layers) {
        for (const layer of layers) {
          if (layer.id.startsWith('place_') || layer.id.startsWith('poi_')) {
            map?.on('click', layer.id, (event) => {
              if (event.features && event.features[0]) {
                mapTouchHandlers
                  .get('poi_click')
                  ?.forEach((value) => value(event));
              }
            });
          }
        }
      }
    });
    window.addEventListener('resize', () => setBottomCardAllowance());
  },
});
</script>
