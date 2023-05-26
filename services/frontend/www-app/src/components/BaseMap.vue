<template>
  <div id="map"></div>
</template>

<style lang="scss">
.headway-ctrl-scale {
  display: flex;
  flex-direction: row;
  align-items: baseline;
  gap: 4px;
  padding-left: 4px;
  cursor: pointer;
}

.headway-ctrl-scale-text {
  color: black;
  text-shadow: 0px 0px 2px white, 0px 0px 2px white, 0px 0px 2px white;
}

.headway-ctrl-scale-ruler {
  height: 4px;
  border: solid black 1.5px;
  border-top: none;
}

.desktop .headway-ctrl-wrapper {
  background: #fffa;
  gap: 8px;
}

.mobile .headway-ctrl-wrapper {
  float: right;
  gap: 16px;
  margin: 8px;
}

.headway-ctrl-wrapper {
  display: flex;
  flex-direction: row;
  align-items: center;
  clear: both;
  font-size: 80%;

  .maplibregl-ctrl {
    margin: 0;
  }

  .maplibregl-ctrl-attrib {
    background: none;
    color: black;
  }
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';
import ScaleControl from 'src/ui/ScaleControl';
import maplibregl, {
  AttributionControl,
  FitBoundsOptions,
  FlyToOptions,
  LayerSpecification,
  LineLayerSpecification,
  LngLat,
  LngLatBounds,
  LngLatBoundsLike,
  LngLatLike,
  MapLayerEventType,
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
import Place, { PlaceId } from 'src/models/Place';
import TripLayerId from 'src/models/TripLayerId';
import env from 'src/utils/env';
import { Platform } from 'quasar';
import WrapperControl from 'src/ui/WrapperControl';

export var map: maplibregl.Map | null = null;
const mapContainerId = 'map';

async function loadMap(): Promise<maplibregl.Map> {
  let initialCenter: LngLatLike = [0, 0];
  let initialZoom = 1;

  const mostRecentMapCenter = Prefs.stored.mostRecentMapCenter;
  if (mostRecentMapCenter) {
    initialCenter = mostRecentMapCenter;
    const mostRecentMapZoom = Prefs.stored.mostRecentMapZoom;
    if (mostRecentMapZoom) {
      initialZoom = Math.min(10, mostRecentMapZoom);
    }
  }

  let mapOptions: MapOptions = {
    container: mapContainerId,
    style: '/tileserver/styles/basic/style.json', // style URL
    center: initialCenter, // starting position [lng, lat]
    zoom: initialZoom, // starting zoom
    attributionControl: false,
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

export interface BaseMapInterface {
  getZoom: () => number;
  getCenter: () => LngLat;
  getBounds: () => LngLatBounds;
  flyTo: (location: LngLatLike, options?: FlyToOptions) => void;
  flyToPlace: (place: Place, options?: FlyToOptions) => void;
  fitBounds: (bounds: LngLatBoundsLike, options?: FitBoundsOptions) => void;
  setCursor: (key: string) => void;
  pushMarker: (key: string, marker: Marker) => void;
  removeMarker: (key: string) => void;
  removeAllMarkers: () => void;
  removeMarkersExcept: (keys: string[]) => void;
  pushLayer: (
    key: TripLayerId,
    source: SourceSpecification,
    layer: LayerSpecification,
    beforeLayerType: string
  ) => void;
  pushTripLayer: (
    layerId: TripLayerId,
    geometry: GeoJSON.Geometry,
    paint: LineLayerSpecification['paint']
  ) => void;
  hasLayer: (layerId: TripLayerId) => boolean;
  removeLayersExcept: (layerIds: TripLayerId[]) => void;
  /// returns wether a layer was removed
  removeLayer: (layerId: TripLayerId) => boolean;
  removeAllLayers(): void;
  on: (
    type: keyof MapLayerEventType,
    layerId: string,
    listener: (ev: unknown) => void
  ) => void;
}

var baseMapMethods: BaseMapInterface | undefined = undefined;

// There really has to be a better way to do this, but we only ever have 1 base map so I guess it works.
export function getBaseMap(): BaseMapInterface | undefined {
  return baseMapMethods;
}

let baseMapPromiseResolver: (baseMap: BaseMapInterface) => void;
export const baseMapPromise = new Promise<BaseMapInterface>((resolver) => {
  baseMapPromiseResolver = resolver;
});

export default defineComponent({
  name: 'BaseMap',
  data: function (): {
    flyToOptions?: FlyToOptions;
    boundsToFit?: LngLatBoundsLike;
    markers: Map<string, Marker>;
    layers: string[];
    loaded: boolean;
    touchHandlers: Map<BaseMapEventType, Array<BaseMapEventHandler>>;
    touchHandlerIdx: number;
  } {
    return {
      flyToOptions: undefined,
      boundsToFit: undefined,
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
    hasLayer(layerId: TripLayerId): boolean {
      return this.layers.includes(layerId.toString());
    },
    removeAllLayers(): void {
      this.removeLayersExcept([]);
    },
    removeLayer(layerId: TripLayerId): boolean {
      const index = this.layers.indexOf(layerId.toString());
      if (index === -1) {
        return false;
      } else {
        this.layers.splice(index, 1);
        this.ensureMapLoaded((map: maplibregl.Map) => {
          map.removeLayer(layerId.toString());
          map.removeSource(layerId.toString());
        });
        return true;
      }
    },
    pushTripLayer(
      layerId: TripLayerId,
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
          id: layerId.toString(),
          type: 'line',
          source: layerId.toString(),
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
      layerId: TripLayerId,
      source: SourceSpecification,
      layer: LayerSpecification,
      beforeLayerType: string
    ) {
      let sourceKey = layerId.toString();
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
      this.ensureMapLoaded((map: maplibregl.Map) => {
        if (map.getLayer(sourceKey)) {
          map.removeLayer(sourceKey);
        }
        if (map.getSource(sourceKey)) {
          map.removeSource(sourceKey);
        }
        map.addSource(sourceKey, source);
        let beforeLayerId = undefined;
        if (beforeLayerType) {
          for (const key in map.style._layers) {
            let layer = map.style._layers[key];
            if (layer.type == beforeLayerType) {
              beforeLayerId = layer.id;
              break;
            }
          }
        }
        map.addLayer(layer, beforeLayerId);
        this.layers.push(layerId.toString());
      });
    },
    removeLayersExcept(keep: TripLayerId[]) {
      const keepStrings = keep.map((layerId) => layerId.toString());
      let newLayers: string[] = [];
      this.layers.forEach((layerId: string) => {
        if (keepStrings.includes(layerId)) {
          if (!newLayers.includes(layerId)) {
            newLayers.push(layerId);
          }
        } else {
          if (map?.getLayer(layerId.toString())) {
            map.removeLayer(layerId.toString());
          }
          if (map?.getSource(layerId.toString())) {
            map.removeSource(layerId.toString());
          }
        }
      });
      this.layers = newLayers;
    },
    setCursor(value: string): void {
      this.ensureMapLoaded((map) => {
        map.getCanvas().style.cursor = value;
      });
    },
    flyToPlace(place: Place, options?: FlyToOptions): void {
      if (place.bbox) {
        const defaultOptions = {
          maxZoom: 16,
        };
        options = { ...defaultOptions, ...(options || {}) };
        // prefer bounds when available so we don't "overzoom" on a large
        // entity like an entire city.
        this.fitBounds(place.bbox, options);
      } else {
        const defaultOptions = {
          maxZoom: 16,
          zoom: 16,
        };
        options = { ...defaultOptions, ...(options || {}) };
        this.flyTo(place.point, options);
      }
    },
    flyTo: function (location: LngLatLike, options: FlyToOptions = {}): void {
      if (this.loaded) {
        options['center'] = location;
        map?.flyTo(options, { flying: true });
      } else {
        this.$data.flyToOptions = options;
      }
    },
    fitBounds: function (
      bounds: LngLatBoundsLike,
      options: FitBoundsOptions = {}
    ): void {
      const defaultOptions = {
        padding: Math.min(window.innerWidth, window.innerHeight) / 8,
      };
      options = { ...defaultOptions, ...(options || {}) };

      if (this.loaded) {
        map?.fitBounds(bounds, options);
      } else {
        this.$data.boundsToFit = bounds;
      }
    },

    //
    // Event Handling
    //

    on(
      type: keyof MapLayerEventType,
      layerId: string,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      listener: (ev: any) => void
    ) {
      this.ensureMapLoaded((map: maplibregl.Map) => {
        map.on(type, layerId.toString(), listener);
      });
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
      getCenter: () => map.getCenter(),
      getBounds: () => map.getBounds(),
      getZoom: () => map.getZoom(),
      setCursor: this.setCursor,
      flyToPlace: this.flyToPlace,
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
      on: this.on,
    };

    // Ironically the "compact" representation takes up a lot more vertical space, since we have other
    // controls in the bottom right, this the compact version is taking up a more contentious resource.
    const attributionControl = new AttributionControl({ compact: false });
    const scaleControl = new ScaleControl({ maxWidth: 120 });
    const geolocate = new maplibregl.GeolocateControl({
      positionOptions: { enableHighAccuracy: true },
      showAccuracyCircle: true,
      showUserLocation: true,
      trackUserLocation: true,
    });
    env.geolocation.register(geolocate);

    const nav = new maplibregl.NavigationControl({
      visualizePitch: true,
      showCompass: true,
      showZoom: true,
    });
    map.addControl(nav, 'top-right');

    if (Platform.is.desktop) {
      let wrapperControl = new WrapperControl();
      wrapperControl.pushChild(scaleControl);
      wrapperControl.pushChild(attributionControl);
      map.addControl(wrapperControl, 'bottom-right');
      map.addControl(geolocate, 'bottom-right');
    } else {
      map.addControl(attributionControl, 'bottom-right');

      let wrapperControl = new WrapperControl();
      wrapperControl.pushChild(scaleControl);
      wrapperControl.pushChild(geolocate);
      map.addControl(wrapperControl, 'bottom-right');
    }

    map.on('load', () => {
      this.loaded = true;
      if (this.flyToOptions) {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        this.flyTo(this.flyToOptions.center!, this.flyToOptions);
        this.flyToOptions = undefined;
      }
      if (this.boundsToFit) {
        this.fitBounds(this.boundsToFit);
        this.boundsToFit = undefined;
      }
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
        Prefs.stored.setMostRecentMapCenter(map.getCenter());
        Prefs.stored.setMostRecentMapZoom(map.getZoom());
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

    map.on('load', () => {
      const layers: LayerSpecification[] = map.getStyle().layers;
      if (!layers) {
        throw new Error('layers must not be empty');
      }

      for (const layer of layers) {
        if (layer.id.startsWith('place_') || layer.id.startsWith('poi_')) {
          this.on('mouseover', layer.id, (event) => {
            if (event.features && event.features[0]) {
              this.setCursor('pointer');
            } else {
              console.warn('hovered place without feature', layer, event);
            }
          });
          this.on('mouseout', layer.id, () => {
            this.setCursor('');
          });
          this.on('click', layer.id, (event) => {
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

      // Add 3-D buildings
      const render3DZoomLevel = 15;
      type LerpableValue =
        | maplibregl.ExpressionSpecification
        | maplibregl.ColorSpecification
        | number;
      function zoomLerp<T>(
        unzoomedValue: LerpableValue,
        zoomedValue: LerpableValue
      ): maplibregl.DataDrivenPropertyValueSpecification<T> {
        return [
          'interpolate',
          ['linear'],
          ['zoom'],
          render3DZoomLevel,
          // This is a bug in maplibregl's types, fixed in 3.0.0 https://github.com/maplibre/maplibre-gl-js/pull/1890
          // we can delete this lint exception after upgrading
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          unzoomedValue as any,
          render3DZoomLevel + 0.5,
          // This is a bug in maplibregl's types, fixed in 3.0.0 https://github.com/maplibre/maplibre-gl-js/pull/1890
          // we can delete this lint exception after upgrading
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          zoomedValue as any,
        ];
      }

      // Rendering in 3D is cute, but it can also be annoying.
      // Introducing a toggle button takes up valuable real estate.
      // So the compromise is to have the 3D effect always on (when zoomed in)
      // but to dampen in a bit so that the arguably "bad" side effects are minimized.
      const heightDampeningFactor = 0.333;
      map.addLayer(
        {
          id: 'subtle_3d_buildings',
          source: 'openmaptiles',
          'source-layer': 'building',
          filter: ['!=', 'hide_3d', true],
          type: 'fill-extrusion',
          minzoom: render3DZoomLevel,
          paint: {
            'fill-extrusion-color': zoomLerp<maplibregl.ColorSpecification>(
              'hsl(35, 8%, 85%)',
              'hsl(35, 8%, 75%)'
            ),
            'fill-extrusion-height':
              zoomLerp<maplibregl.ExpressionSpecification>(0, [
                '*',
                ['get', 'render_height'],
                heightDampeningFactor,
                // This is a bug in maplibregl's types, fixed in 3.0.0 https://github.com/maplibre/maplibre-gl-js/pull/1890
                // we can delete this lint exception after upgrading
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
              ]) as any,
            'fill-extrusion-base': zoomLerp<maplibregl.ExpressionSpecification>(
              0,
              ['*', ['get', 'render_min_height'], heightDampeningFactor]
              // This is a bug in maplibregl's types, fixed in 3.0.0 https://github.com/maplibre/maplibre-gl-js/pull/1890
              // we can delete this lint exception after upgrading
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
            ) as any,
          },
        },
        // add 3-d building layer behind any symbol layer
        layers.find(
          (layer) => layer.type === 'symbol' && layer.layout?.['text-field']
        )?.id
      );

      if (!baseMapMethods) {
        throw new Error('baseMapMethods must remain set');
      }
      baseMapPromiseResolver(baseMapMethods);
    });
  },
});
</script>
