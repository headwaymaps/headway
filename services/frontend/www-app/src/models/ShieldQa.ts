import { MercatorCoordinate } from 'maplibre-gl';
import { expression, latest } from '@maplibre/maplibre-gl-style-spec';
import type { StylePropertyExpression } from '@maplibre/maplibre-gl-style-spec';
import type {
  ExpressionSpecification,
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
export const SHIELD_QA_ZOOM = 14;
export const LABEL_WIDTH = 330;
export const COLUMN_WIDTH = 64;
export const ROW_HEIGHT = 36;

const UNKNOWN_NETWORK = 'XX:none';
const EXIT_ROW = 'exit (road_exit_shield)';
export const SHIELD_QA_SOURCE = 'shield-qa';
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

/** A table of every road shield, drawn by the style's own shield layers. */
export default class ShieldQa {
  constructor(
    private base: StyleSpecification,
    private roadShield: SymbolLayerSpecification,
    private exitShield: SymbolLayerSpecification,
  ) {}

  private expressions = new Map<SizingProperty, StylePropertyExpression>();
  private commonMetrics: ShieldMetrics | undefined;

  static fromStyle(style: StyleSpecification): ShieldQa {
    const symbolLayer = (id: string): SymbolLayerSpecification => {
      const layer = style.layers.find((l) => l.id === id);
      if (layer?.type !== 'symbol') {
        throw new Error(`style has no symbol layer "${id}"`);
      }
      return layer;
    };
    return new ShieldQa(
      style,
      symbolLayer('road_shield'),
      symbolLayer('road_exit_shield'),
    );
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

  private shieldProperties(network: string, ref: string) {
    return { network: 'road', route_1_network: network, route_1_ref: ref, ref };
  }

  rows(filter: string): string[] {
    const needle = filter.trim().toLowerCase();
    return [...this.networks(), UNKNOWN_NETWORK, EXIT_ROW].filter((row) =>
      row.toLowerCase().includes(needle),
    );
  }

  style(rows: string[]): StyleSpecification {
    const background = this.base.layers.filter((l) => l.type === 'background');
    const shieldLayer = (
      layer: SymbolLayerSpecification,
    ): SymbolLayerSpecification => {
      return {
        id: layer.id,
        type: 'symbol',
        source: SHIELD_QA_SOURCE,
        filter: layer.filter,
        paint: layer.paint,
        layout: {
          ...layer.layout,
          'symbol-placement': 'point',
          'icon-allow-overlap': true,
          'text-allow-overlap': true,
        },
      };
    };
    return {
      version: 8,
      sprite: this.base.sprite,
      glyphs: this.base.glyphs,
      sources: {
        [SHIELD_QA_SOURCE]: { type: 'geojson', data: this.features(rows) },
      },
      layers: [
        ...background,
        shieldLayer(this.roadShield),
        shieldLayer(this.exitShield),
      ],
    };
  }

  features(rows: string[]): GeoJSON.FeatureCollection<GeoJSON.Point> {
    const features: GeoJSON.Feature<GeoJSON.Point>[] = [];
    rows.forEach((row, rowIdx) => {
      const y = (rowIdx + 0.5) * ROW_HEIGHT;
      SHIELD_QA_REFS.forEach((ref, colIdx) => {
        const x = LABEL_WIDTH + (colIdx + 0.5) * COLUMN_WIDTH;
        const properties =
          row === EXIT_ROW
            ? { subclass: 'junction', ref, ref_length: ref.length }
            : this.shieldProperties(row, ref);
        features.push(pointAt(x, y, properties));
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
