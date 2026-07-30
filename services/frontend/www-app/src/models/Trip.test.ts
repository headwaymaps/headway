import { describe, expect, test } from 'vitest';
import Trip from './Trip';
import {
  TransitVehicleMode,
  TravelmuxItinerary,
  TravelmuxLeg,
  TravelmuxMode,
} from 'src/services/TravelmuxClient';
import { DistanceUnits } from 'src/utils/models';

function leg(mode: TravelmuxMode, distanceMeters: number): TravelmuxLeg {
  const transit = mode === TravelmuxMode.Transit;
  return {
    mode,
    geometry: '',
    fromPlace: { lat: 47.5, lon: -122.3, name: 'from' },
    toPlace: { lat: 47.6, lon: -122.3, name: 'to' },
    startTime: '2024-05-17T12:35:01-07:00',
    endTime: '2024-05-17T12:45:01-07:00',
    distanceMeters,
    durationSeconds: 600,
    transitLeg: transit
      ? {
          vehicleMode: TransitVehicleMode.Bus,
          route: { shortName: '40' },
          realTime: false,
          alerts: [],
        }
      : undefined,
    nonTransitLeg: transit
      ? undefined
      : { maneuvers: [], substantialStreetNames: [] },
  } as TravelmuxLeg;
}

function trip(legs: TravelmuxLeg[]): Trip {
  const itinerary: TravelmuxItinerary = {
    mode: TravelmuxMode.Transit,
    startTime: '2024-05-17T12:35:01-07:00',
    endTime: '2024-05-17T13:05:01-07:00',
    durationSeconds: 1800,
    distanceMeters: legs.reduce((total, l) => total + l.distanceMeters, 0),
    bounds: { min: [-122.3, 47.5], max: [-122.3, 47.6] },
    legs,
  };
  return new Trip(itinerary, DistanceUnits.Kilometers);
}

describe('nonTransitDistanceMeters', () => {
  test('sums the walking legs of a transit itinerary', () => {
    const t = trip([
      leg(TravelmuxMode.Walk, 696.33),
      leg(TravelmuxMode.Transit, 5809.19),
      leg(TravelmuxMode.Walk, 222.24),
    ]);
    expect(t.nonTransitDistanceMeters).toBeCloseTo(918.57);
    expect(t.withBicycle).toBe(false);
    expect(t.walkingDistanceFormatted).toEqual('0.9 km walk total');
  });

  test('sums the cycling legs of a bike+transit itinerary', () => {
    const t = trip([
      leg(TravelmuxMode.Bike, 2478.0),
      leg(TravelmuxMode.Transit, 4003.0),
      leg(TravelmuxMode.Bike, 1029.0),
    ]);
    expect(t.nonTransitDistanceMeters).toBeCloseTo(3507.0);
    expect(t.withBicycle).toBe(true);
    expect(t.walkingDistanceFormatted).toEqual('3.5 km bike total');
  });
});

describe('legs', () => {
  test('names transit legs by route, non-transit legs by mode', () => {
    const t = trip([
      leg(TravelmuxMode.Walk, 100),
      leg(TravelmuxMode.Transit, 5000),
    ]);
    const [walk, bus] = t.legs;
    expect(walk?.transitLeg).toBe(false);
    expect(walk?.shortName).toEqual('🚶‍♀️');
    expect(bus?.transitLeg).toBe(true);
    expect(bus?.shortName).toEqual('🚍 40');
  });

  test('parses times', () => {
    const t = trip([leg(TravelmuxMode.Walk, 100)]);
    expect(t.legs[0]?.startTime.toISOString()).toEqual(
      '2024-05-17T19:35:01.000Z',
    );
  });
});
