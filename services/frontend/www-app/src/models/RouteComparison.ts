import { LngLat, LngLatBounds, Marker } from 'maplibre-gl';
import type { LineLayerSpecification, Map as MaplibreMap } from 'maplibre-gl';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import TripLayerId from './TripLayerId';

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

/// One route's stretches that aren't on every route, and where to label them.
export type RouteDifference = {
  letter: string;
  color: string;
  /// On this route alone.
  solo: GeoJSON.MultiLineString;
  /// On some of the other routes too.
  partlyShared: GeoJSON.MultiLineString;
  /// The middle of each of those stretches.
  labels: LngLat[];
  /// Where this route's labels sit from their anchors, in pixels, clear of other routes' labels.
  labelOffset: [number, number];
};

/// Where a set of routes, each a line of coordinates, agree and diverge.
export default class RouteComparison {
  /// On every route.
  readonly common: GeoJSON.MultiLineString;
  readonly routes: RouteDifference[];

  constructor(lines: Position[][]) {
    const counts = new Map<string, number>();
    for (const line of lines) {
      for (const key of new Set(edgeKeys(line))) {
        counts.set(key, (counts.get(key) ?? 0) + 1);
      }
    }
    const everyRoute = (key: string) => counts.get(key) === lines.length;
    const alone = (key: string) => !everyRoute(key) && counts.get(key) === 1;

    this.common = multiLine(runs(lines[0] ?? [], everyRoute));
    this.routes = lines.map((line, i) => {
      const solo = runs(line, alone);
      const partlyShared = runs(line, (k) => !everyRoute(k) && !alone(k));
      return {
        letter: String.fromCharCode('A'.charCodeAt(0) + i),
        color: COLORS[i % COLORS.length]!,
        solo: multiLine(solo),
        partlyShared: multiLine(partlyShared),
        labels: [...solo, ...partlyShared].map(midpoint),
        labelOffset: LABEL_OFFSETS[i % LABEL_OFFSETS.length]!,
      };
    });
  }

  /// Draws what the routes share in gray over the routes, then each one's differences in its color.
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
    if (routes.length > 1) {
      push(TripLayerId.comparedCommon(), this.common, {
        'line-color': COMMON_COLOR,
        'line-width': 7,
      });
    }
    routes.forEach((route, i) => {
      const width = i === selectedIdx ? 7 : 5;
      push(TripLayerId.comparedSolo(i), route.solo, {
        'line-color': route.color,
        'line-width': width,
      });
      push(TripLayerId.comparedPartlyShared(i), route.partlyShared, {
        'line-color': route.color,
        'line-width': width,
        'line-offset': this.lineOffset(i),
      });
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
      multiLine([
        ...this.common.coordinates,
        ...routes.flatMap((route) => route.solo.coordinates),
      ]),
      0,
    );
    routes.forEach((route, i) =>
      pushChevrons(
        baseMap,
        TripLayerId.comparedPartlySharedChevrons(i),
        route.partlyShared,
        this.lineOffset(i),
      ),
    );
  }

  /// How far, in pixels, route `i` sits to the side of a stretch it shares with others.
  private lineOffset(i: number): number {
    return (i - (this.routes.length - 1) / 2) * 5;
  }

  /// Around everything that isn't common to all the routes, if anything is.
  get differenceBounds(): LngLatBounds | undefined {
    const positions = this.routes.flatMap((route) =>
      [route.solo, route.partlyShared].flatMap((m) => m.coordinates.flat()),
    );
    if (positions.length === 0) return undefined;
    const bounds = new LngLatBounds();
    for (const [lng, lat] of positions) bounds.extend([lng!, lat!]);
    return bounds;
  }
}

/// Fine chevrons along `geometry`, pointing the way it runs, `offset` pixels to its right.
function pushChevrons(
  baseMap: BaseMapInterface,
  id: TripLayerId,
  geometry: GeoJSON.MultiLineString,
  offset: number,
) {
  baseMap.removeLayer(id);
  baseMap.pushLayer(
    id,
    {
      type: 'geojson',
      data: { type: 'Feature', properties: {}, geometry },
    },
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
        'icon-offset': [0, offset],
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

function edgeKey(a: Position, b: Position): string {
  return [a, b]
    .map(([lng, lat]) => `${lng!.toFixed(6)},${lat!.toFixed(6)}`)
    .sort()
    .join('|');
}

function edgeKeys(line: Position[]): string[] {
  return line.slice(1).map((position, i) => edgeKey(line[i]!, position));
}

/// The contiguous stretches of `line` whose edges all satisfy `include`.
function runs(
  line: Position[],
  include: (key: string) => boolean,
): Position[][] {
  const result: Position[][] = [];
  let run: Position[] | undefined;
  edgeKeys(line).forEach((key, i) => {
    if (!include(key)) {
      run = undefined;
      return;
    }
    if (!run) {
      run = [line[i]!];
      result.push(run);
    }
    run.push(line[i + 1]!);
  });
  return result;
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
