import { expect, test } from '@jest/globals';
import { fastDistanceMeters, fastPolylineDistanceMeters } from './geomath';

test('Zero fast distance at 0, 0', () => {
  expect(fastDistanceMeters({ long: 0, lat: 0 }, { long: 0, lat: 0 })).toBe(0);
});

test('Zero fast distance at 30, 30', () => {
  expect(fastDistanceMeters({ long: 30, lat: 30 }, { long: 30, lat: 30 })).toBe(
    0
  );
});

test('E/W fast distance at 0, 0', () => {
  expect(
    fastDistanceMeters({ long: 0, lat: 0 }, { long: 0.0001, lat: 0 })
  ).toBeCloseTo(11.12);
});

test('N/S fast distance at 0, 0', () => {
  expect(
    fastDistanceMeters({ long: 0, lat: 0.0001 }, { long: 0, lat: 0 })
  ).toBeCloseTo(11.12);
});

test('Diagonal fast distance at 0, 0', () => {
  expect(
    fastDistanceMeters({ long: 0, lat: 0.0001 }, { long: -0.0001, lat: 0 })
  ).toBeCloseTo(15.73);
});

test('E/W fast distance at 30, 30', () => {
  expect(
    fastDistanceMeters({ long: 30, lat: 30 }, { long: 30.0001, lat: 30 })
  ).toBeCloseTo(9.63);
});

test('N/S fast distance at 30, 30', () => {
  expect(
    fastDistanceMeters({ long: 30, lat: 30.0001 }, { long: 30, lat: 30 })
  ).toBeCloseTo(11.12);
});

test('Diagonal fast distance at 30, 30', () => {
  expect(
    fastDistanceMeters({ long: 30, lat: 30.0001 }, { long: 29.9999, lat: 30 })
  ).toBeCloseTo(14.71);
});

test('Empty polyline distance zero', () => {
  expect(fastPolylineDistanceMeters([])).toBe(0);
});

test('Single-point polyline distance zero', () => {
  expect(fastPolylineDistanceMeters([{ long: 70, lat: 70 }])).toBe(0);
});

test('Colocated polyline distance zero', () => {
  expect(
    fastPolylineDistanceMeters([
      { long: 70, lat: -70 },
      { long: 70, lat: -70 },
    ])
  ).toBe(0);
});

test('Singe-segment polyline distance', () => {
  expect(
    fastPolylineDistanceMeters([
      { long: 30, lat: 30 },
      { long: 30.0001, lat: 30 },
    ])
  ).toBeCloseTo(9.63);
});

test('Two-segment polyline distance', () => {
  expect(
    fastPolylineDistanceMeters([
      { long: 30, lat: 30 },
      { long: 30.0001, lat: 30 },
      { long: 30.0002, lat: 30.0001 },
    ])
  ).toBeCloseTo(24.34);
});
