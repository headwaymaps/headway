<template>
  <div id="map"></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import maplibregl, {
  LayerSpecification,
  LngLatBoundsLike,
  LngLatLike,
  MapMouseEvent,
  Marker,
  SourceSpecification,
} from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import Prefs from 'src/utils/Prefs';
import { debounce } from 'lodash';

export var map: maplibregl.Map | null = null;

async function loadMap() {
  let initialCenter: LngLatLike = [0, 0];
  let initialZoom = 1;

  const mostRecentMapCenter = Prefs.stored().mostRecentMapCenter();
  if (mostRecentMapCenter) {
    initialCenter = mostRecentMapCenter;
    const mostRecentMapZoom = Prefs.stored().mostRecentMapZoom();
    if (mostRecentMapZoom) {
      initialZoom = Math.min(10, mostRecentMapZoom);
    }
  }

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
      center: initialCenter, // starting position [lng, lat]
      zoom: initialZoom, // starting zoom
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
      center: initialCenter, // starting position [lng, lat]
      maxBounds: maxBounds,
      zoom: initialZoom, // starting zoom
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

/**
 * Polyfill for geolocation permission
 */
async function geolocationPermissionState(): Promise<string> {
  if (navigator.permissions === undefined) {
    // assume "unknown" on platforms like Safari 15 which don't
    // support the permissions API.
    return 'prompt';
  } else {
    const result = await navigator.permissions.query({
      name: 'geolocation',
    });
    return result.state;
  }
}

export interface BaseMapInterface {
  flyTo: (location: [number, number], zoom: number) => void;
  fitBounds: (bounds: LngLatBoundsLike) => void;
  pushMarker: (key: string, marker: Marker) => void;
  removeMarkersExcept: (keys: string[]) => void;
  pushLayer: (
    key: string,
    source: SourceSpecification,
    layer: LayerSpecification
  ) => void;
  removeLayersExcept: (keys: string[]) => void;
}

var baseMapMethods: BaseMapInterface | undefined = undefined;

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
    markers: Map<string, Marker>;
    layers: string[];
    loaded: boolean;
  } {
    return {
      flyToLocation: undefined,
      boundsToFit: undefined,
      hasGeolocated: false,
      markers: new Map(),
      layers: [],
      loaded: false,
    };
  },
  methods: {
    ensureMapLoaded(fn: (map: maplibregl.Map) => void) {
      const mapCapture = map;
      if (mapCapture && this.loaded) {
        fn(mapCapture);
      } else if (mapCapture) {
        mapCapture.on('load', () => fn(mapCapture));
      }
    },
    pushMarker(key: string, marker: Marker) {
      let oldMarker = this.markers.get(key);
      if (oldMarker) {
        oldMarker.remove();
      }
      this.markers.set(key, marker);
      this.ensureMapLoaded((map) => marker.addTo(map));
    },
    removeMarkersExcept(keys: string[]) {
      this.markers.forEach((marker, key) => {
        if (keys.indexOf(key) === -1) {
          marker.remove();
        }
      });
    },
    pushLayer(
      key: string,
      source: SourceSpecification,
      layer: LayerSpecification
    ) {
      let sourceKey = `headway_custom_layer_${key}`;
      let actualLayer = layer;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      if ((actualLayer as any).source) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (actualLayer as any).source = sourceKey;
      }
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      if ((actualLayer as any).id) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (actualLayer as any).id = sourceKey;
      }
      this.ensureMapLoaded((map) => {
        if (map.getLayer(sourceKey)) {
          map.removeLayer(sourceKey);
        }
        if (map.getSource(sourceKey)) {
          map.removeSource(sourceKey);
        }
        map.addSource(sourceKey, source);
        map.addLayer(layer);
        this.layers.push(sourceKey);
      });
    },
    removeLayersExcept(keys: string[]) {
      this.layers.forEach((key) => {
        if (keys.indexOf(key) === -1) {
          if (map?.getLayer(key)) {
            map.removeLayer(key);
          }
          if (map?.getSource(key)) {
            map.removeSource(key);
          }
        }
      });
      this.layers = keys;
    },
    flyTo: async function (location: [number, number], zoom: number) {
      const permissionState = await geolocationPermissionState();
      if (this.$data.hasGeolocated === true || permissionState !== 'granted') {
        map?.flyTo({ center: location, zoom: zoom }, { flying: true });
      } else {
        this.$data.flyToLocation = { center: location, zoom: zoom };
      }
    },
    fitBounds: async function (
      bounds: LngLatBoundsLike,
      options: FitBoundsOptions = {}
    ) {
      const permissionState = await geolocationPermissionState();
      const defaultOptions = {
        padding: Math.min(window.innerWidth, window.innerHeight) / 8,
      };
      options = { ...defaultOptions, ...(options || {}) };

      if (this.$data.hasGeolocated === true || permissionState !== 'granted') {
        map?.fitBounds(bounds, options);
      } else {
        this.$data.boundsToFit = bounds;
      }
    },
  },
  mounted: async function () {
    await loadMap();
    // This might be the ugliest thing in this whole web app. Expose methods through an internal thing.
    baseMapMethods = {
      flyTo: this.flyTo,
      fitBounds: this.fitBounds,
      pushMarker: this.pushMarker,
      removeMarkersExcept: this.removeMarkersExcept,
      pushLayer: this.pushLayer,
      removeLayersExcept: this.removeLayersExcept,
    };
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
    map?.on('load', () => {
      this.loaded = true;
    });
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

    map?.on(
      'moveend',
      debounce(() => {
        Prefs.stored().setMostRecentMapCenter(map.getCenter());
        Prefs.stored().setMostRecentMapZoom(map.getZoom());
      }, 2000)
    );
    setTimeout(async () => {
      const permissionState = await geolocationPermissionState();
      if (permissionState === 'granted') {
        map?.on('load', () => {
          geolocate.trigger();
          geolocate.on('geolocate', () => {
            if (!this.$data.hasGeolocated) {
              // prevent the default "zoom" that occurs when we automatically `trigger`
              // the geolocate button.
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
