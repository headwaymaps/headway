import { expect, test } from 'vitest';
import polyline from '@mapbox/polyline';
import RouteComparison from './RouteComparison';

/// Encodes `[lng, lat]` positions as travelmux does.
function encode(...positions: number[][]): string {
  return polyline.encode(
    positions.map(([lng, lat]) => [lat!, lng!]),
    6,
  );
}

const start = [0, 0];
const a = [1, 0];
const detourX = [1, 1];
const detourY = [2, 1];
const b = [2, 0];
const end = [3, 0];

test('a shared route with one detour', () => {
  const comparison = new RouteComparison({
    common: [encode(start, a), encode(b, end)],
    routes: [
      { itineraries: [0], solo: [encode(a, b)], partlyShared: [] },
      {
        itineraries: [1],
        solo: [encode(a, detourX, detourY, b)],
        partlyShared: [],
      },
    ],
  });
  expect(comparison.common.coordinates).toEqual([
    [start, a],
    [b, end],
  ]);
  expect(comparison.routes[1]!.solo.coordinates).toEqual([
    [a, detourX, detourY, b],
  ]);
  expect(comparison.routes.map((route) => route.letter)).toEqual(['A', 'B']);
  expect(comparison.differenceBounds?.toArray()).toEqual([
    [1, 0],
    [2, 1],
  ]);
});

test('every diverging stretch is labeled at its middle', () => {
  const comparison = new RouteComparison({
    common: [],
    routes: [
      {
        itineraries: [0],
        solo: [encode(start, a)],
        partlyShared: [{ geometry: encode(b, end), routes: [0, 1] }],
      },
      {
        itineraries: [1],
        solo: [],
        partlyShared: [{ geometry: encode(b, end), routes: [0, 1] }],
      },
    ],
  });
  expect(comparison.routes[0]!.labels.map((l) => l.toArray())).toEqual([
    [0.5, 0],
    [2.5, 0],
  ]);
  expect(comparison.routes[0]!.labelOffset).not.toEqual(
    comparison.routes[1]!.labelOffset,
  );
});

test('a lone route has no differences', () => {
  const comparison = new RouteComparison({
    common: [encode(start, a, end)],
    routes: [{ itineraries: [0], solo: [], partlyShared: [] }],
  });
  expect(comparison.routes[0]!.labels).toEqual([]);
  expect(comparison.differenceBounds).toBeUndefined();
});

test('routes sharing a stretch sit side by side, whatever their letters', () => {
  const shared = { geometry: encode(a, b), routes: [0, 3] };
  const alone = { itineraries: [1], solo: [encode(a, b)], partlyShared: [] };
  const comparison = new RouteComparison({
    common: [],
    routes: [
      { itineraries: [0], solo: [], partlyShared: [shared] },
      alone,
      { ...alone, itineraries: [2] },
      { itineraries: [3], solo: [], partlyShared: [shared] },
    ],
  });
  const offset = (route: number) =>
    comparison.routes[route]!.partlyShared[0]!.offset;
  expect([offset(0), offset(3)]).toEqual([-2.5, 2.5]);
});
