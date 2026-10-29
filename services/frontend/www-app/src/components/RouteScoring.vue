<template>
  <div v-if="status" class="route-scoring-status">{{ status }}</div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, toRaw, watch } from 'vue';
import polyline from '@mapbox/polyline';
import { Point } from 'maplibre-gl';
import type { GeoJSONSource } from 'maplibre-gl';
import { baseMapPromise, map as baseMap } from 'src/components/BaseMap.vue';
import type Trip from 'src/models/Trip';
import { TravelmuxMode } from 'src/services/TravelmuxClient';
import type {
  TravelmuxItinerary,
  TravelmuxLeg,
} from 'src/services/TravelmuxClient';
import { hoverDetails } from 'src/utils/debug';
import { ValhallaBikeCosting } from 'src/utils/ValhallaBikeFactor';
import type {
  ValhallaBikeEdge,
  ValhallaBikeFactor,
} from 'src/utils/ValhallaBikeFactor';

const props = defineProps<{ trip?: Trip }>();

const VALHALLA_PATH = '/valhalla';
const COSTING: Record<TravelmuxMode, string | undefined> = {
  [TravelmuxMode.Bike]: 'bicycle',
  [TravelmuxMode.Walk]: 'pedestrian',
  [TravelmuxMode.Drive]: 'auto',
  [TravelmuxMode.Transit]: undefined,
};
/// The bicycle costing_options travelmux plans with, so the scoring matches its routes.
const TRAVELMUX_BIKE_OPTIONS = {
  road_class_factors: [20, 8, 1.25, 1, 0.5, 0.25, 0.25, 1],
  bicycle_network_factor: 0.275,
  use_roads: 0.1,
  avoid_bad_surfaces: 0.5,
  walk_bike_factor: 3,
  destination_only_penalty: 300,
  gate_penalty: 150,
};
const EDGE_WALK_FAILED = 443;
/// How close, in pixels, the cursor must be to an edge to show its scoring.
const HOVER_PIXELS = 10;
const HOVERED_SOURCE = 'route-scoring-hovered';

type LonLat = [number, number];
type ShapeRange = { begin_shape_index: number; end_shape_index: number };
// incomplete
type TraceEdge = ValhallaBikeEdge &
  ShapeRange & {
    names?: string[];
    way_id: number;
    end_node: { elapsed_cost: number; elapsed_time: number };
  };
/// One Valhalla edge, with the cost of reaching its end from its start, turn included.
type Segment = {
  line: LonLat[];
  cost: number;
  time: number;
  edge: TraceEdge;
  /// Our rebuild of the bike costing's factor; null for other modes.
  model: ValhallaBikeFactor | null;
};

const status = ref<string>();
let segments: Segment[] = [];
let traceSeq = 0;

// Selecting the selected trip again hands us a new proxy of the same itinerary.
watch(
  () => props.trip && toRaw(props.trip.raw),
  (itinerary) => void score(itinerary),
  { immediate: true },
);

async function score(itinerary: TravelmuxItinerary | undefined) {
  const seq = ++traceSeq;
  segments = [];
  clearHighlight();
  status.value = undefined;
  const leg = itinerary?.legs.length === 1 ? itinerary.legs[0] : undefined;
  const costing = leg && COSTING[leg.mode];
  if (!itinerary || !leg?.nonTransitLeg || !costing) return;
  if (itinerary.routers?.[0] !== 'valhalla') {
    status.value = 'Route scoring: only Valhalla routes are scored.';
    return;
  }

  status.value = 'Route scoring: tracing…';
  const line = polyline
    .decode(leg.geometry, 6)
    .map(([lat, lon]) => [lon, lat] as LonLat);
  let edges: TraceEdge[];
  try {
    edges = await traceEdges(costing, leg, line);
  } catch (e) {
    if (seq === traceSeq) status.value = `Route scoring: ${e}`;
    return;
  }
  if (seq !== traceSeq) return;

  const bikeCosting =
    leg.mode === TravelmuxMode.Bike
      ? new ValhallaBikeCosting(TRAVELMUX_BIKE_OPTIONS)
      : null;
  segments = edges.map((edge, i) => {
    const start = edges[i - 1]?.end_node ?? {
      elapsed_cost: 0,
      elapsed_time: 0,
    };
    return {
      line: line.slice(edge.begin_shape_index, edge.end_shape_index + 1),
      cost: edge.end_node.elapsed_cost - start.elapsed_cost,
      time: edge.end_node.elapsed_time - start.elapsed_time,
      edge,
      model: bikeCosting?.factor(edge) ?? null,
    };
  });
  status.value =
    segments.length > 0
      ? undefined
      : "Route scoring: Valhalla couldn't trace this route.";
}

/// Where each maneuver starts and ends along `line`, which its start point lies on.
function maneuverRanges(leg: TravelmuxLeg, line: LonLat[]): ShapeRange[] {
  const starts: number[] = [];
  for (const { startPoint } of leg.nonTransitLeg!.maneuvers) {
    const from = starts[starts.length - 1] ?? 0;
    const index = line.findIndex(
      ([lon, lat], i) =>
        i >= from &&
        Math.abs(lon - startPoint[0]) < 1e-6 &&
        Math.abs(lat - startPoint[1]) < 1e-6,
    );
    if (index >= 0) starts.push(index);
  }
  return starts.map((begin, i) => ({
    begin_shape_index: begin,
    end_shape_index: starts[i + 1] ?? line.length - 1,
  }));
}

/// The edges Valhalla walks along the leg, which share its shape indices. Where Valhalla can't
/// walk its own route, as at some traffic circles, it walks each maneuver from the start of the
/// one before, so the turn into it is costed, and skips the maneuvers it still can't walk.
async function traceEdges(
  costing: string,
  leg: TravelmuxLeg,
  line: LonLat[],
): Promise<TraceEdge[]> {
  const whole = await traceShape(costing, leg.geometry);
  if (whole) return whole;

  const points = line.map(([lon, lat]) => [lat, lon] as LonLat);
  const maneuvers = maneuverRanges(leg, line).filter(
    (m) => m.end_shape_index > m.begin_shape_index,
  );
  const pieces = await Promise.all(
    maneuvers.map(async (m, i) => {
      const from = maneuvers[i - 1]?.begin_shape_index ?? m.begin_shape_index;
      const edges = await traceShape(
        costing,
        polyline.encode(points.slice(from, m.end_shape_index + 1), 6),
      );
      return (edges ?? [])
        .map((edge, j) => ({
          edge: {
            ...edge,
            begin_shape_index: edge.begin_shape_index + from,
            end_shape_index: edge.end_shape_index + from,
          },
          previous: edges![j - 1]?.end_node,
        }))
        .filter(
          ({ edge }) =>
            edge.begin_shape_index >= m.begin_shape_index &&
            edge.begin_shape_index < m.end_shape_index,
        );
    }),
  );
  const edges: TraceEdge[] = [];
  for (const { edge, previous } of pieces.flat()) {
    const before = edges[edges.length - 1]?.end_node ?? {
      elapsed_cost: 0,
      elapsed_time: 0,
    };
    const start = previous ?? { elapsed_cost: 0, elapsed_time: 0 };
    edges.push({
      ...edge,
      end_node: {
        elapsed_cost:
          before.elapsed_cost + edge.end_node.elapsed_cost - start.elapsed_cost,
        elapsed_time:
          before.elapsed_time + edge.end_node.elapsed_time - start.elapsed_time,
      },
    });
  }
  return edges;
}

/// Null when Valhalla can't walk `shape` edge by edge.
async function traceShape(
  costing: string,
  shape: string,
): Promise<TraceEdge[] | null> {
  const resp = await fetch(`${VALHALLA_PATH}/trace_attributes`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      encoded_polyline: shape,
      shape_match: 'edge_walk',
      costing,
      costing_options:
        costing === 'bicycle' ? { bicycle: TRAVELMUX_BIKE_OPTIONS } : {},
      units: 'kilometers',
    }),
  });
  const json = await resp.json();
  if (json.error_code === EDGE_WALK_FAILED) return null;
  if (!resp.ok)
    throw new Error(`trace_attributes: ${json.error ?? resp.status}`);
  return json.edges;
}

function distanceToSegment(p: Point, a: Point, b: Point): number {
  const ab = b.sub(a);
  const lengthSquared = ab.x * ab.x + ab.y * ab.y;
  const t =
    lengthSquared === 0
      ? 0
      : Math.max(
          0,
          Math.min(
            1,
            ((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / lengthSquared,
          ),
        );
  return p.dist(a.add(ab.mult(t)));
}

function segmentAt(point: Point): Segment | undefined {
  const map = baseMap!;
  let nearest: Segment | undefined;
  let nearestPixels = HOVER_PIXELS;
  for (const segment of segments) {
    const projected = segment.line.map((lonLat) => map.project(lonLat));
    for (let i = 1; i < projected.length; i++) {
      const pixels = distanceToSegment(point, projected[i - 1]!, projected[i]!);
      if (pixels < nearestPixels) {
        nearest = segment;
        nearestPixels = pixels;
      }
    }
  }
  return nearest;
}

function setHovered(line: LonLat[] | null) {
  const map = baseMap!;
  const data: GeoJSON.GeoJSON = line
    ? {
        type: 'Feature',
        properties: {},
        geometry: { type: 'LineString', coordinates: line },
      }
    : { type: 'FeatureCollection', features: [] };
  const source = map.getSource<GeoJSONSource>(HOVERED_SOURCE);
  if (source) {
    source.setData(data);
    return;
  }
  map.addSource(HOVERED_SOURCE, { type: 'geojson', data });
  map.addLayer({
    id: HOVERED_SOURCE,
    type: 'line',
    source: HOVERED_SOURCE,
    paint: { 'line-color': '#f03e3e', 'line-width': 8, 'line-opacity': 0.6 },
    layout: { 'line-cap': 'round' },
  });
}

function clearHighlight() {
  if (baseMap?.getSource(HOVERED_SOURCE)) setHovered(null);
}

/// The scoring of the edge at `point`, which is highlighted on the map.
function detailsAt(point: Point): HTMLElement | undefined {
  const segment = segmentAt(point);
  setHovered(segment?.line ?? null);
  return segment && segmentTable(segment);
}

/// Cost per second of travel, i.e. how much Valhalla dislikes it beyond its time.
const costRate = (cost: number, time: number) =>
  time > 0 ? `×${(cost / time).toFixed(2)}` : '';

/// Which of the popup's sections are open, kept as you move between segments.
const openSections = new Set(['cost', 'factor']);

/// A popup section that remembers whether it's open.
function collapsible(key: string, title: string, content: Node): HTMLElement {
  const details = document.createElement('details');
  details.open = openSections.has(key);
  details.addEventListener('toggle', () => {
    if (details.open) openSections.add(key);
    else openSections.delete(key);
  });
  const summary = document.createElement('summary');
  summary.textContent = title;
  details.append(summary, content);
  return details;
}

/// One collapsible section per aspect of the segment.
function segmentTable(s: Segment): HTMLElement {
  const root = document.createElement('div');
  root.className = 'segment-popup';

  const section = (
    key: string,
    title: string,
    rows: [string, string | Node][],
  ) => {
    const table = document.createElement('table');
    for (const [label, value] of rows) {
      const row = table.insertRow();
      row.insertCell().textContent = label;
      row.insertCell().append(value);
    }
    root.append(collapsible(key, title, table));
  };

  const fixed = (n: number, digits = 2) => n.toFixed(digits);
  const model = s.model;
  section('cost', 'Cost', [
    ['cost', fixed(s.cost, 1)],
    ['time', `${fixed(s.time, 1)} s`],
    ['cost / time', costRate(s.cost, s.time)],
    ['turn ≈', model ? fixed(s.cost - model.factor * model.seconds, 1) : '–'],
  ]);

  if (model) {
    section('factor', 'Factor (modelled)', [
      ['factor', `×${fixed(model.factor)}`],
      ...model.terms.map(
        (term) => [`+ ${term.label}`, fixed(term.value)] as [string, string],
      ),
      ['accommodation', `×${fixed(model.accommodation)}`],
      ['network', `×${fixed(model.network)}`],
      ['speed penalty', `×${fixed(model.speedPenalty)}`],
    ]);
  }

  const way = document.createElement('a');
  way.href = `https://www.openstreetmap.org/way/${s.edge.way_id}`;
  way.target = '_blank';
  way.textContent = String(s.edge.way_id);
  section('edge', 'Edge', [
    ['name', s.edge.names?.join(' / ') || '–'],
    ['length', `${Math.round(s.edge.length * 1000)} m`],
    ['use', s.edge.use],
    ['class', s.edge.road_class],
    ['cycle lane', s.edge.cycle_lane ?? '–'],
    ['shoulder', s.edge.shoulder ? 'yes' : 'no'],
    ['lanes', String(s.edge.lane_count ?? '–')],
    ['surface', s.edge.surface],
    ['grade', `${s.edge.weighted_grade}%`],
    ['bike network', String(s.edge.bicycle_network ?? 0)],
    ['car speed', `${s.edge.speeds_non_faded.no_flow} kph`],
    ['way', way],
  ]);

  return root;
}

let unmounted = false;
onMounted(async () => {
  await baseMapPromise;
  if (unmounted) return;
  hoverDetails.value = detailsAt;
});

onBeforeUnmount(() => {
  unmounted = true;
  traceSeq++;
  if (hoverDetails.value === detailsAt) hoverDetails.value = null;
  if (baseMap?.getLayer(HOVERED_SOURCE)) baseMap.removeLayer(HOVERED_SOURCE);
  if (baseMap?.getSource(HOVERED_SOURCE)) baseMap.removeSource(HOVERED_SOURCE);
});
</script>

<style lang="scss" scoped>
.route-scoring-status {
  font-size: 12px;
  color: #666;
  padding: 0 8px;
  margin-bottom: 8px;
}
</style>

<style lang="scss">
.segment-popup {
  font-size: 12px;
  color: black;
  line-height: 1.4;

  summary {
    cursor: pointer;
    font-weight: 600;
    margin-top: 4px;
  }
  table {
    border-collapse: collapse;
    margin: 2px 0 4px;
  }
  th,
  td {
    padding: 1px 6px;
    text-align: right;
    vertical-align: top;
  }
  th {
    font-weight: 600;
  }
  td:first-child {
    text-align: left;
    color: #555;
    white-space: nowrap;
  }
}
</style>
