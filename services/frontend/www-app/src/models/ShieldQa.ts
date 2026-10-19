import { MercatorCoordinate } from 'maplibre-gl';
import { expression, latest } from '@maplibre/maplibre-gl-style-spec';
import type { StylePropertyExpression } from '@maplibre/maplibre-gl-style-spec';
import type {
  ExpressionSpecification,
  LayerSpecification,
  SourceSpecification,
  StyleSpecification,
  SymbolLayerSpecification,
} from 'maplibre-gl';

export const SHIELD_QA_REFS = [
  '5',
  '90',
  '509',
  '28A',
  '1604',
  'E470',
  'PE-1N',
  '344M',
  'G1501',
  '123456',
];
type ShieldQaColumn = { label: string; ref: string; noText?: boolean };

export const SHIELD_QA_COLUMNS: ShieldQaColumn[] = [
  ...SHIELD_QA_REFS.map((ref) => ({ label: ref, ref })),
  { label: 'no text', ref: '5', noText: true },
];
export const SHIELD_QA_ZOOM = 14;
export const LABEL_WIDTH = 330;
export const COLUMN_WIDTH = 64;
export const ROW_HEIGHT = 36;

const UNKNOWN_NETWORK = 'XX:none';
export const SHIELD_QA_SOURCE = 'shield-qa';
const SCENERY_SOURCE_PREFIX = 'shield-qa-scenery-';
/** Wider than any window, so the scenery runs off the right edge. */
const SCENERY_WIDTH = 4000;
/** Of the base style's source layers, the ones the scenery draws. */
type ScenerySourceLayer = 'landuse' | 'park' | 'transportation';
const WORLD_SIZE = 512 * 2 ** SHIELD_QA_ZOOM;
const LONGEST_REF = '123456';

// The style-spec's reference JSON is wider than the type it accepts.
type PropertySpec = Parameters<typeof expression.createPropertyExpression>[2];

type SizingProperty = 'text-offset' | 'text-size' | 'icon-size';

/** How a network's road_shield differs from the style default and is sized. */
type ShieldMetrics = {
  offset: string;
  textCap: string;
  textFit: string;
  iconSize: string;
};

/** A table of every road shield, drawn by the style's own road_shield layer. */
export default class ShieldQa {
  constructor(
    private base: StyleSpecification,
    private roadShield: SymbolLayerSpecification,
  ) {}

  private expressions = new Map<SizingProperty, StylePropertyExpression>();
  private commonMetrics: ShieldMetrics | undefined;

  static fromStyle(style: StyleSpecification): ShieldQa {
    const roadShield = style.layers.find((l) => l.id === 'road_shield');
    if (roadShield?.type !== 'symbol') {
      throw new Error('style has no symbol layer "road_shield"');
    }
    return new ShieldQa(style, roadShield);
  }

  /** Every route_1_network the road_shield layer draws a blank for. */
  networks(): string[] {
    const networks = new Set<string>();
    collectRouteNetworks(this.roadShield.layout?.['icon-image'], networks);
    return [...networks].sort();
  }

  /** Where this network's shield differs from the style default or common sizing. */
  adjustments(network: string): string {
    const metrics = this.metrics(network);
    const common = this.common();
    const notes = [];
    if (metrics.offset !== 'none') {
      notes.push(`offset ${metrics.offset}`);
    }
    if (metrics.textCap !== common.textCap) {
      notes.push(`text ${metrics.textCap}`);
    }
    if (metrics.textFit !== 'none') {
      notes.push(`fit ${metrics.textFit}`);
    }
    if (metrics.iconSize !== common.iconSize) {
      notes.push(`icon ×${metrics.iconSize}`);
    }
    return notes.join(' · ');
  }

  private common(): ShieldMetrics {
    if (!this.commonMetrics) {
      const all = this.networks().map((network) => this.metrics(network));
      const mostCommon = (key: keyof ShieldMetrics): string => {
        const counts = new Map<string, number>();
        for (const metrics of all) {
          counts.set(metrics[key], (counts.get(metrics[key]) ?? 0) + 1);
        }
        return [...counts].sort((a, b) => b[1] - a[1])[0]![0];
      };
      this.commonMetrics = {
        offset: mostCommon('offset'),
        textCap: mostCommon('textCap'),
        textFit: mostCommon('textFit'),
        iconSize: mostCommon('iconSize'),
      };
    }
    return this.commonMetrics;
  }

  private metrics(network: string): ShieldMetrics {
    const [x, y] = this.evaluate('text-offset', network, '5') as number[];
    const [defaultX, defaultY] = this.evaluate(
      'text-offset',
      UNKNOWN_NETWORK,
      '5',
    ) as number[];
    const textCap = this.evaluate('text-size', network, '5') as number;
    const longestSize = this.evaluate(
      'text-size',
      network,
      LONGEST_REF,
    ) as number;
    const textFit = fitWidth(textCap, longestSize);
    const defaultTextCap = this.evaluate(
      'text-size',
      UNKNOWN_NETWORK,
      '5',
    ) as number;
    const defaultLongestSize = this.evaluate(
      'text-size',
      UNKNOWN_NETWORK,
      LONGEST_REF,
    ) as number;
    return {
      offset: offsetArrows(x! - defaultX!, y! - defaultY!),
      textCap: round(textCap),
      textFit: fitDelta(textFit, fitWidth(defaultTextCap, defaultLongestSize)),
      iconSize: round(this.evaluate('icon-size', network, '5') as number),
    };
  }

  private evaluate(
    property: SizingProperty,
    network: string,
    ref: string,
  ): unknown {
    let parsed = this.expressions.get(property);
    if (!parsed) {
      const result = expression.createPropertyExpression(
        this.roadShield.layout?.[property],
        'layout',
        latest.layout_symbol[property] as unknown as PropertySpec,
      );
      if (result.result !== 'success') {
        throw new Error(`can't parse road_shield ${property}`);
      }
      parsed = result.value;
      this.expressions.set(property, parsed);
    }
    return parsed.evaluate(
      { zoom: SHIELD_QA_ZOOM },
      { type: 1, properties: this.shieldProperties(network, ref) },
    );
  }

  private shieldProperties(network: string, ref: string, noText = false) {
    return {
      network: 'road',
      route_1_network: network,
      route_1_ref: ref,
      ref,
      shield_qa_no_text: noText,
    };
  }

  rows(filter: string): string[] {
    const needle = filter.trim().toLowerCase();
    return [...this.networks(), UNKNOWN_NETWORK].filter((row) =>
      row.toLowerCase().includes(needle),
    );
  }

  style(rows: string[]): StyleSpecification {
    const background = this.base.layers.filter((l) => l.type === 'background');
    const scenery = this.scenery();
    const sceneryLayers = this.base.layers.flatMap((layer) =>
      this.sceneryLayer(layer, scenery),
    );
    const scenerySources: Record<string, SourceSpecification> = {};
    for (const [sourceLayer, data] of Object.entries(scenery)) {
      scenerySources[SCENERY_SOURCE_PREFIX + sourceLayer] = {
        type: 'geojson',
        data,
      };
    }
    const roadShield: SymbolLayerSpecification = {
      id: this.roadShield.id,
      type: 'symbol',
      source: SHIELD_QA_SOURCE,
      filter: this.roadShield.filter,
      paint: this.roadShield.paint,
      layout: {
        ...this.roadShield.layout,
        'symbol-placement': 'point',
        'icon-allow-overlap': true,
        'text-allow-overlap': true,
        'text-field': [
          'case',
          ['get', 'shield_qa_no_text'],
          '',
          this.roadShield.layout?.['text-field'] ?? '',
        ] as ExpressionSpecification,
      },
    };
    return {
      version: 8,
      sprite: this.base.sprite,
      glyphs: this.base.glyphs,
      sources: {
        ...scenerySources,
        [SHIELD_QA_SOURCE]: { type: 'geojson', data: this.features(rows) },
      },
      layers: [...background, ...sceneryLayers, roadShield],
    };
  }

  /** `layer` drawing the scenery instead of tiles, if it draws land or roads. */
  private sceneryLayer(
    layer: LayerSpecification,
    scenery: Record<ScenerySourceLayer, GeoJSON.FeatureCollection>,
  ): LayerSpecification[] {
    if (layer.type !== 'fill' && layer.type !== 'line') {
      return [];
    }
    const sourceLayer = layer['source-layer'];
    if (!sourceLayer || !(sourceLayer in scenery)) {
      return [];
    }
    const sceneryLayer = {
      ...layer,
      source: SCENERY_SOURCE_PREFIX + sourceLayer,
    };
    delete sceneryLayer['source-layer'];
    return [sceneryLayer];
  }

  /**
   * A neighbourhood for the shields to sit on, as the style's own tiles would describe it: each row's
   * shields ride a motorway, over houses, the odd park, and streets running between the columns.
   */
  scenery(): Record<ScenerySourceLayer, GeoJSON.FeatureCollection> {
    const rowCount = this.rows('').length;
    const height = rowCount * ROW_HEIGHT;
    const columnXs = Array.from(
      { length: Math.ceil((SCENERY_WIDTH - LABEL_WIDTH) / COLUMN_WIDTH) + 1 },
      (_, idx) => LABEL_WIDTH + idx * COLUMN_WIDTH,
    );

    const streets = columnXs.map((x, idx) =>
      lineAt(
        [
          [x, 0],
          [x, height],
        ],
        { class: idx % 4 === 0 ? 'secondary' : 'minor' },
      ),
    );
    const motorways = Array.from({ length: rowCount }, (_, row) => {
      const y = (row + 0.5) * ROW_HEIGHT;
      return lineAt(
        [
          [0, y],
          [SCENERY_WIDTH, y],
        ],
        { class: 'motorway' },
      );
    });

    // A block sits between two streets and two motorways; a few of them are parks.
    const inset = 4;
    const parks: GeoJSON.Feature[] = [];
    for (let row = 0; row < rowCount - 1; row++) {
      columnXs.slice(0, -1).forEach((x, column) => {
        if ((row * 7 + column * 3) % 11 === 0) {
          const top = (row + 0.5) * ROW_HEIGHT;
          parks.push(
            rectAt(
              x + inset,
              top + inset,
              x + COLUMN_WIDTH - inset,
              top + ROW_HEIGHT - inset,
              {},
            ),
          );
        }
      });
    }

    return {
      landuse: collection([
        rectAt(0, 0, SCENERY_WIDTH, height, { class: 'residential' }),
      ]),
      park: collection(parks),
      transportation: collection([...streets, ...motorways]),
    };
  }

  features(rows: string[]): GeoJSON.FeatureCollection<GeoJSON.Point> {
    const features: GeoJSON.Feature<GeoJSON.Point>[] = [];
    rows.forEach((row, rowIdx) => {
      const y = (rowIdx + 0.5) * ROW_HEIGHT;
      SHIELD_QA_COLUMNS.forEach(({ ref, noText }, colIdx) => {
        const x = LABEL_WIDTH + (colIdx + 0.5) * COLUMN_WIDTH;
        features.push(pointAt(x, y, this.shieldProperties(row, ref, noText)));
      });
    });
    return { type: 'FeatureCollection', features };
  }
}

/** The location of a pixel in the table, measured from its top-left corner. */
export function tablePixelToLngLat(x: number, y: number): [number, number] {
  const lngLat = new MercatorCoordinate(
    0.5 + x / WORLD_SIZE,
    0.5 + y / WORLD_SIZE,
  ).toLngLat();
  return [lngLat.lng, lngLat.lat];
}

function offsetArrows(x: number, y: number): string {
  const parts = [];
  if (x !== 0) {
    parts.push(`${x < 0 ? '←' : '→'}${round(Math.abs(x))}`);
  }
  if (y !== 0) {
    parts.push(`${y < 0 ? '↑' : '↓'}${round(Math.abs(y))}`);
  }
  return parts.length ? `${parts.join(' ')}em` : 'none';
}

function round(n: number): string {
  return String(Math.round(n * 100) / 100);
}

function fitWidth(textCap: number, longestSize: number): number | undefined {
  return longestSize < textCap
    ? Number(round(longestSize * LONGEST_REF.length))
    : undefined;
}

function fitDelta(
  fit: number | undefined,
  defaultFit: number | undefined,
): string {
  if (fit === defaultFit) {
    return 'none';
  }
  if (fit === undefined || defaultFit === undefined) {
    return fit === undefined ? 'none' : `+${round(fit)}`;
  }
  const delta = fit - defaultFit;
  return `${delta > 0 ? '+' : ''}${round(delta)}`;
}

function collection(features: GeoJSON.Feature[]): GeoJSON.FeatureCollection {
  return { type: 'FeatureCollection', features };
}

function lineAt(
  points: [number, number][],
  properties: GeoJSON.GeoJsonProperties,
): GeoJSON.Feature<GeoJSON.LineString> {
  return {
    type: 'Feature',
    geometry: {
      type: 'LineString',
      coordinates: points.map(([x, y]) => tablePixelToLngLat(x, y)),
    },
    properties,
  };
}

function rectAt(
  left: number,
  top: number,
  right: number,
  bottom: number,
  properties: GeoJSON.GeoJsonProperties,
): GeoJSON.Feature<GeoJSON.Polygon> {
  const corners: [number, number][] = [
    [left, top],
    [right, top],
    [right, bottom],
    [left, bottom],
    [left, top],
  ];
  return {
    type: 'Feature',
    geometry: {
      type: 'Polygon',
      coordinates: [corners.map(([x, y]) => tablePixelToLngLat(x, y))],
    },
    properties,
  };
}

function pointAt(
  x: number,
  y: number,
  properties: GeoJSON.GeoJsonProperties,
): GeoJSON.Feature<GeoJSON.Point> {
  return {
    type: 'Feature',
    geometry: { type: 'Point', coordinates: tablePixelToLngLat(x, y) },
    properties,
  };
}

function collectRouteNetworks(expression: unknown, networks: Set<string>) {
  if (!Array.isArray(expression)) {
    return;
  }
  if (isRouteNetworkMatch(expression)) {
    // ["match", input, label, output, label, output, ..., fallback]
    for (let i = 2; i < expression.length - 1; i += 2) {
      const label: string | string[] = expression[i];
      for (const network of [label].flat()) {
        networks.add(network);
      }
    }
  }
  for (const child of expression) {
    collectRouteNetworks(child, networks);
  }
}

function isRouteNetworkMatch(expression: unknown[]): boolean {
  const input = expression[1] as ExpressionSpecification | undefined;
  return (
    expression[0] === 'match' &&
    Array.isArray(input) &&
    input[0] === 'get' &&
    input[1] === 'route_1_network'
  );
}
