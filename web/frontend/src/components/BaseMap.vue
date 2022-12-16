<template>
  <div id="map"></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import maplibregl, {
  FitBoundsOptions,
  LayerSpecification,
  LineLayerSpecification,
  LngLatBoundsLike,
  LngLatLike,
  MapMouseEvent,
  MapOptions,
  Marker,
  SourceSpecification,
} from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import Prefs from 'src/utils/Prefs';
import Config from 'src/utils/Config';
import { mapFeatureToPlace } from 'src/utils/models';
import { debounce } from 'lodash';
import { PlaceId } from 'src/models/Place';

export var map: maplibregl.Map | null = null;
const mapContainerId = 'map';

async function loadMap(): Promise<maplibregl.Map> {
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

  let mapOptions: MapOptions = {
    container: mapContainerId,
    style: '/styles/style/style.json', // style URL
    center: initialCenter, // starting position [lng, lat]
    zoom: initialZoom, // starting zoom
  };

  let bounds = Config.maxBounds;
  if (bounds) {
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
    mapOptions.maxBounds = maxBounds;
  }

  map = new maplibregl.Map(mapOptions);
  return map;
}

const mapTouchTimeouts: NodeJS.Timeout[] = [];

type BaseMapEventType = 'click' | 'longpress' | 'poi_click';
type BaseMapEventHandler = (
  event: MapMouseEvent & {
    features?: GeoJSON.Feature[] | undefined;
  }
) => void;

function clearAllTimeouts() {
  mapTouchTimeouts.forEach((timeout) => clearTimeout(timeout));
  mapTouchTimeouts.length = 0;
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
  flyTo: (location: LngLatLike, zoom: number) => Promise<void>;
  fitBounds: (bounds: LngLatBoundsLike, options?: FitBoundsOptions) => void;
  pushMarker: (key: string, marker: Marker) => void;
  removeMarker: (key: string) => void;
  removeAllMarkers: () => void;
  removeMarkersExcept: (keys: string[]) => void;
  pushLayer: (
    key: string,
    source: SourceSpecification,
    layer: LayerSpecification,
    beforeLayerType: string
  ) => void;
  pushTripLayer: (
    layerId: string,
    geometry: GeoJSON.Geometry,
    paint: LineLayerSpecification['paint']
  ) => void;
  removeLayersExcept: (keys: string[]) => void;
  /// returns wether a layer was removed
  removeLayer: (key: string) => boolean;
  removeAllLayers(): void;
  hasLayer: (key: string) => boolean;
}

var baseMapMethods: BaseMapInterface | undefined = undefined;

// There really has to be a better way to do this, but we only ever have 1 base map so I guess it works.
export function getBaseMap() {
  return baseMapMethods;
}

export default defineComponent({
  name: 'BaseMap',
  data: function (): {
    flyToLocation: { center: LngLatLike; zoom: number } | undefined;
    boundsToFit: LngLatBoundsLike | undefined;
    hasGeolocated: boolean;
    markers: Map<string, Marker>;
    layers: string[];
    loaded: boolean;
    touchHandlers: Map<BaseMapEventType, Array<BaseMapEventHandler>>;
    touchHandlerIdx: number;
  } {
    return {
      flyToLocation: undefined,
      boundsToFit: undefined,
      hasGeolocated: false,
      markers: new Map(),
      layers: [],
      loaded: false,
      touchHandlers: new Map(),
      touchHandlerIdx: 0,
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
    removeMarker(key: string): boolean {
      let marker = this.markers.get(key);
      if (marker) {
        this.markers.delete(key);
        marker.remove();
        return true;
      } else {
        return false;
      }
    },
    removeAllMarkers() {
      this.markers.forEach((marker) => marker.remove());
      this.markers = new Map();
    },
    removeMarkersExcept(keys: string[]) {
      this.markers.forEach((marker, key) => {
        if (keys.indexOf(key) === -1) {
          marker.remove();
          this.markers.delete(key);
        }
      });
    },
    hasLayer(key: string): boolean {
      return this.layers.includes(key);
    },
    removeAllLayers(): void {
      this.removeLayersExcept([]);
    },
    removeLayer(key: string): boolean {
      const index = this.layers.indexOf(key);
      if (index === -1) {
        return false;
      } else {
        this.layers.splice(index, 1);
        this.ensureMapLoaded((map) => {
          map.removeLayer(key);
          map.removeSource(key);
        });
        return true;
      }
    },
    pushTripLayer(
      layerId: string,
      geometry: GeoJSON.Geometry,
      paint: LineLayerSpecification['paint']
    ): void {
      this.pushLayer(
        layerId,
        {
          type: 'geojson',
          data: {
            type: 'Feature',
            properties: {},
            geometry,
          },
        },
        {
          id: layerId,
          type: 'line',
          source: layerId,
          layout: {
            'line-join': 'round',
            'line-cap': 'round',
          },
          paint,
        },
        'symbol'
      );
    },
    pushLayer(
      key: string,
      source: SourceSpecification,
      layer: LayerSpecification,
      beforeLayerType: string
    ) {
      let sourceKey = key;
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
        let beforeLayerId = undefined;
        if (beforeLayerType) {
          for (key in map.style._layers) {
            let layer = map.style._layers[key];
            if (layer.type == beforeLayerType) {
              beforeLayerId = layer.id;
              break;
            }
          }
        }
        map.addLayer(layer, beforeLayerId);
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
    flyTo: async function (location: LngLatLike, zoom: number): Promise<void> {
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
    pushTouchHandler: function (
      event: BaseMapEventType,
      handler: BaseMapEventHandler
    ): void {
      let eventList = this.touchHandlers.get(event);
      if (!eventList) {
        eventList = [];
        this.touchHandlers.set(event, eventList);
      }
      eventList.push(handler);
    },
  },
  mounted: async function () {
    let map = await loadMap();
    // This might be the ugliest thing in this whole web app. Expose methods through an internal thing.
    baseMapMethods = {
      flyTo: this.flyTo,
      fitBounds: this.fitBounds,
      pushMarker: this.pushMarker,
      removeMarker: this.removeMarker,
      removeAllMarkers: this.removeAllMarkers,
      removeMarkersExcept: this.removeMarkersExcept,
      pushLayer: this.pushLayer,
      pushTripLayer: this.pushTripLayer,
      hasLayer: this.hasLayer,
      removeLayer: this.removeLayer,
      removeLayersExcept: this.removeLayersExcept,
      removeAllLayers: this.removeAllLayers,
    };
    var nav = new maplibregl.NavigationControl({
      visualizePitch: true,
      showCompass: true,
      showZoom: true,
    });
    map.addControl(nav, 'top-right');
    var geolocate = new maplibregl.GeolocateControl({
      positionOptions: { enableHighAccuracy: true },
      showAccuracyCircle: true,
      showUserLocation: true,
    });
    map.addControl(geolocate, 'bottom-right');
    map.on('load', () => {
      this.loaded = true;
    });
    map.on('click', (event: MapMouseEvent) => {
      this.touchHandlers.get('click')?.forEach((value) => value(event));
    });
    map.on('mousedown', (event: MapMouseEvent) => {
      clearAllTimeouts();
      mapTouchTimeouts.push(
        setTimeout(() => {
          this.touchHandlers.get('longpress')?.forEach((value) => value(event));
        }, 700)
      );
    });
    map.on('touchstart', (event: MapMouseEvent) => {
      clearAllTimeouts();
      mapTouchTimeouts.push(
        setTimeout(() => {
          this.touchHandlers.get('longpress')?.forEach((value) => value(event));
        }, 700)
      );
    });
    map.on('mouseup', () => clearAllTimeouts());
    map.on('mousemove', () => clearAllTimeouts());
    map.on('touchup', () => clearAllTimeouts());
    map.on('touchend', () => clearAllTimeouts());
    map.on('move', () => clearAllTimeouts());
    map.on(
      'moveend',
      debounce(() => {
        Prefs.stored().setMostRecentMapCenter(map.getCenter());
        Prefs.stored().setMostRecentMapZoom(map.getZoom());
      }, 2000)
    );

    const mapElement = document.getElementById(mapContainerId);
    if (!mapElement) {
      console.error('mapElement not found');
      return;
    }
    new ResizeObserver(() => {
      // This seems more robust than using maplibre's built-in trackResize.
      // I think maybe trackResize only works when the browser resizes, but we
      // also resize our map element frequently (on mobile especially), in
      // order to fit the content before and after the map.
      // Without this resize, I was finding that "fitBounds" would provide
      // incorrect results.
      map.resize();
    }).observe(mapElement);

    this.pushTouchHandler('longpress', (event) => {
      const id = PlaceId.location(event.lngLat);
      this.$router.push({
        name: 'place',
        params: { placeId: id.serialized() },
      });
    });
    this.pushTouchHandler('poi_click', async (event) => {
      if (!event.features) {
        console.warn('poi_click without features');
        return;
      }
      let place = await mapFeatureToPlace(event?.features[0]);
      if (place?.id.gid) {
        const id = PlaceId.gid(place.id.gid);
        this.$router.push({
          name: 'place',
          params: { placeId: id.serialized() },
        });
      } else {
        // There are certain OSM features that fail to reverse-geocode - maybe OSM
        // entities which aren't in pelias? In that case, we just use the lng/lat
        // so the person can still get routing directions to it.
        console.warn(
          'Could not canonicalize map feature, falling back to lon/lat'
        );
        let id = PlaceId.location(event.lngLat);
        this.$router.push({
          name: 'place',
          params: { placeId: id.serialized() },
        });
      }
    });

    setTimeout(async () => {
      const permissionState = await geolocationPermissionState();
      if (permissionState === 'granted') {
        map.on('load', () => {
          geolocate.trigger();
          geolocate.on('geolocate', () => {
            if (!this.$data.hasGeolocated) {
              // prevent the default "zoom" that occurs when we automatically `trigger`
              // the geolocate button.
              map.stop();
              if (this.$data.flyToLocation) {
                map.flyTo(this.$data.flyToLocation, { flying: true });
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
    map.on('load', () => {
      const layers = map.getStyle().layers;
      if (layers) {
        for (const layer of layers) {
          if (layer.id.startsWith('place_') || layer.id.startsWith('poi_')) {
            map.on('mouseenter', layer.id, (event) => {
              if (event.features && event.features[0]) {
                map.getCanvas().style.cursor = 'pointer';
              } else {
                console.warn('hovered place without feature', layer, event);
              }
            });
            map.on('mouseleave', layer.id, () => {
              map.getCanvas().style.cursor = '';
            });
            map.on('click', layer.id, (event) => {
              if (event.features && event.features[0]) {
                this.touchHandlers
                  .get('poi_click')
                  ?.forEach((value) => value(event));
              } else {
                console.warn('clicked place without feature', layer, event);
              }
            });
          }
        }
      }
    });
  },
});

export function sourceMarker(): Marker {
  let element = document.createElement('div');
  element.innerHTML =
    '<svg display="block" height="20" width="20"><circle cx="10" cy="10" r="7" stroke="#111" stroke-width="2" fill="white" /></svg>';
  return new Marker({ element });
}

export function destinationMarker(): Marker {
  return new Marker({ color: '#111111' });
}
</script>
