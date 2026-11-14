import { LngLat, LngLatBounds, Marker } from 'maplibre-gl';
import type { LineLayerSpecification, Map as MaplibreMap } from 'maplibre-gl';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import TripLayerId from './TripLayerId';
import polyline from '@mapbox/polyline';
import type { TravelmuxComparison } from 'src/services/TravelmuxClient';

type Position = GeoJSON.Position;

const COMMON_COLOR = '#6b7280';
const CHEVRON_IMAGE = 'route-chevron';

const LABEL_OFFSETS: [number, number][] = [
  [-14, -14],
  [14, -14],
  [0, 18],
  [-18, 4],
  [18, 4],
  [0, -22],
];

const COLORS = [
  '#0072B2',
  '#D55E00',
  '#009E73',
  '#CC79A7',
  '#E69F00',
  '#56B4E9',
];

/// Pixels between routes drawn side by side along a stretch they share.
const SIDE_BY_SIDE_SPACING = 5;

/// A stretch this route shares with some of the others.
export type SharedStretch = {
  coordinates: Position[];
  /// How far, in pixels, this route sits to the side of the stretch, beside the others on it.
  offset: number;
};

/// One route's stretches that aren't on every route, and where to label them.
export type RouteDifference = {
  letter: string;
  color: string;
  /// On this route alone.
  solo: GeoJSON.MultiLineString;
  /// On some of the other routes too.
  partlyShared: SharedStretch[];
  /// The middle of each of those stretches.
  labels: LngLat[];
  /// Where this route's labels sit from their anchors, in pixels, clear of other routes' labels.
  labelOffset: [number, number];
};

/// Where the routes several routers planned for one trip agree and diverge.
export default class RouteComparison {
  /// On every route.
  readonly common: GeoJSON.MultiLineString;
  readonly routes: RouteDifference[];

  constructor(comparison: TravelmuxComparison) {
    this.common = decodeLines(comparison.common);
    this.routes = comparison.routes.map((route, i) => {
      const solo = decodeLines(route.solo);
      const partlyShared = route.partlyShared.map((stretch) => ({
        coordinates: decodeLine(stretch.geometry),
        offset:
          (stretch.routes.indexOf(i) - (stretch.routes.length - 1) / 2) *
          SIDE_BY_SIDE_SPACING,
      }));
      return {
        letter: String.fromCharCode('A'.charCodeAt(0) + i),
        color: COLORS[i % COLORS.length]!,
        solo,
        partlyShared,
        labels: [
          ...solo.coordinates,
          ...partlyShared.map((stretch) => stretch.coordinates),
        ].map(midpoint),
        labelOffset: LABEL_OFFSETS[i % LABEL_OFFSETS.length]!,
      };
    });
  }

  /// Draws what the routes share in gray, then each one's differences in its color.
  draw(baseMap: BaseMapInterface, map: MaplibreMap, selectedIdx: number) {
    const routes = this.routes;
    const push = (
      id: TripLayerId,
      geometry: GeoJSON.Geometry,
      paint: LineLayerSpecification['paint'],
    ) => {
      baseMap.removeLayer(id);
      baseMap.pushTripLayer(id, geometry, paint);
    };
    push(TripLayerId.comparedCommon(), this.common, {
      'line-color': COMMON_COLOR,
      'line-width': 7,
    });
    routes.forEach((route, i) => {
      const width = i === selectedIdx ? 7 : 5;
      push(TripLayerId.comparedSolo(i), route.solo, {
        'line-color': route.color,
        'line-width': width,
      });
      pushOffsetLines(
        baseMap,
        TripLayerId.comparedPartlyShared(i),
        route.partlyShared,
        { 'line-color': route.color, 'line-width': width },
      );
      route.labels.forEach((label, j) => {
        const element = document.createElement('div');
        element.className = 'route-letter route-letter-marker';
        element.style.background = route.color;
        element.textContent = route.letter;
        baseMap.pushMarker(
          `route_letter_${i}_${j}`,
          new Marker({ element, offset: route.labelOffset }).setLngLat(label),
        );
      });
    });

    if (!map.hasImage(CHEVRON_IMAGE)) {
      map.addImage(CHEVRON_IMAGE, chevronImage(), { pixelRatio: 2 });
    }
    pushChevrons(
      baseMap,
      TripLayerId.comparedChevrons(),
      [
        ...this.common.coordinates,
        ...routes.flatMap((route) => route.solo.coordinates),
      ].map((coordinates) => ({ coordinates, offset: 0 })),
    );
    routes.forEach((route, i) =>
      pushChevrons(
        baseMap,
        TripLayerId.comparedPartlySharedChevrons(i),
        route.partlyShared,
      ),
    );
  }

  /// Around everything that isn't common to all the routes, if anything is.
  get differenceBounds(): LngLatBounds | undefined {
    const positions = this.routes.flatMap((route) => [
      ...route.solo.coordinates.flat(),
      ...route.partlyShared.flatMap((stretch) => stretch.coordinates),
    ]);
    if (positions.length === 0) return undefined;
    const bounds = new LngLatBounds();
    for (const [lng, lat] of positions) bounds.extend([lng!, lat!]);
    return bounds;
  }
}

/// Each stretch as a line, its offset pixels to the right of where it runs.
function offsetFeatures(stretches: SharedStretch[]): GeoJSON.FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: stretches.map(({ coordinates, offset }) => ({
      type: 'Feature',
      properties: { offset, iconOffset: [0, offset] },
      geometry: { type: 'LineString', coordinates },
    })),
  };
}

function pushOffsetLines(
  baseMap: BaseMapInterface,
  id: TripLayerId,
  stretches: SharedStretch[],
  paint: LineLayerSpecification['paint'],
) {
  baseMap.removeLayer(id);
  baseMap.pushLayer(
    id,
    { type: 'geojson', data: offsetFeatures(stretches) },
    {
      id: id.toString(),
      type: 'line',
      source: id.toString(),
      layout: { 'line-join': 'round', 'line-cap': 'round' },
      paint: { ...paint, 'line-offset': ['get', 'offset'] },
    },
    'symbol',
  );
}

/// Fine chevrons along each stretch, pointing the way it runs, beside it by its offset.
function pushChevrons(
  baseMap: BaseMapInterface,
  id: TripLayerId,
  stretches: SharedStretch[],
) {
  baseMap.removeLayer(id);
  baseMap.pushLayer(
    id,
    { type: 'geojson', data: offsetFeatures(stretches) },
    {
      id: id.toString(),
      type: 'symbol',
      source: id.toString(),
      layout: {
        'symbol-placement': 'line',
        'symbol-spacing': 28,
        'icon-image': CHEVRON_IMAGE,
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'icon-rotation-alignment': 'map',
        'icon-offset': ['get', 'iconOffset'],
      },
      paint: { 'icon-opacity': 0.8 },
    },
    'symbol',
  );
}

/// A white chevron pointing east, the way a line symbol's image runs along its line.
function chevronImage(): ImageData {
  const size = 16;
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = size;
  const context = canvas.getContext('2d')!;
  context.strokeStyle = 'white';
  context.lineWidth = 2;
  context.lineCap = 'round';
  context.lineJoin = 'round';
  context.beginPath();
  context.moveTo(5, 3);
  context.lineTo(11, 8);
  context.lineTo(5, 13);
  context.stroke();
  return context.getImageData(0, 0, size, size);
}

function decodeLine(line: string): Position[] {
  return polyline.toGeoJSON(line, 6).coordinates;
}

function decodeLines(polylines: string[]): GeoJSON.MultiLineString {
  return multiLine(polylines.map(decodeLine));
}

function multiLine(coordinates: Position[][]): GeoJSON.MultiLineString {
  return { type: 'MultiLineString', coordinates };
}

function toLngLat([lng, lat]: Position): LngLat {
  return new LngLat(lng!, lat!);
}

function meters(line: Position[]): number {
  return line
    .slice(1)
    .reduce((sum, p, i) => sum + toLngLat(line[i]!).distanceTo(toLngLat(p)), 0);
}

function midpoint(line: Position[]): LngLat {
  let remaining = meters(line) / 2;
  for (let i = 1; i < line.length; i++) {
    const from = toLngLat(line[i - 1]!);
    const to = toLngLat(line[i]!);
    const length = from.distanceTo(to);
    if (length >= remaining && length > 0) {
      const t = remaining / length;
      return new LngLat(
        from.lng + (to.lng - from.lng) * t,
        from.lat + (to.lat - from.lat) * t,
      );
    }
    remaining -= length;
  }
  return toLngLat(line[0]!);
}
