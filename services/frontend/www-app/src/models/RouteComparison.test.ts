import { expect, test } from 'vitest';
import RouteComparison from './RouteComparison';

const start = [0, 0];
const a = [1, 0];
const detourX = [1, 1];
const detourY = [2, 1];
const b = [2, 0];
const end = [3, 0];

test('a shared route with one detour', () => {
  const comparison = new RouteComparison([
    [start, a, b, end],
    [start, a, detourX, detourY, b, end],
  ]);
  expect(comparison.common.coordinates).toEqual([
    [start, a],
    [b, end],
  ]);
  expect(comparison.routes[0]!.solo.coordinates).toEqual([[a, b]]);
  expect(comparison.routes[1]!.solo.coordinates).toEqual([
    [a, detourX, detourY, b],
  ]);
  expect(comparison.routes.map((route) => route.letter)).toEqual(['A', 'B']);
  expect(comparison.differenceBounds?.toArray()).toEqual([
    [1, 0],
    [2, 1],
  ]);
});

test('a stretch on some but not all routes', () => {
  const comparison = new RouteComparison([
    [start, a, b, end],
    [start, a, b, detourY, end],
    [start, detourX, detourY, end],
  ]);
  expect(comparison.common.coordinates).toEqual([]);
  expect(comparison.routes[0]!.partlyShared.coordinates).toEqual([
    [start, a, b],
  ]);
  expect(comparison.routes[1]!.solo.coordinates).toEqual([[b, detourY]]);
  expect(comparison.routes[2]!.partlyShared.coordinates).toEqual([
    [detourY, end],
  ]);
});

test('edges match in either direction', () => {
  const comparison = new RouteComparison([
    [start, a],
    [a, start],
  ]);
  expect(comparison.routes[0]!.solo.coordinates).toEqual([]);
  expect(comparison.differenceBounds).toBeUndefined();
});

test('every diverging stretch is labeled at its middle', () => {
  const comparison = new RouteComparison([
    [start, a, b, end],
    [start, detourX, a, b, detourY, end],
  ]);
  expect(comparison.routes[1]!.labels.length).toBe(2);
  expect(comparison.routes[0]!.labels.map((l) => l.toArray())).toEqual([
    [0.5, 0],
    [2.5, 0],
  ]);
  expect(comparison.routes[0]!.labelOffset).not.toEqual(
    comparison.routes[1]!.labelOffset,
  );
});

test('a lone route has no differences', () => {
  const comparison = new RouteComparison([[start, a, end]]);
  expect(comparison.common.coordinates).toEqual([[start, a, end]]);
  expect(comparison.routes[0]!.solo.coordinates).toEqual([]);
  expect(comparison.routes[0]!.labels).toEqual([]);
  expect(comparison.differenceBounds).toBeUndefined();
});
