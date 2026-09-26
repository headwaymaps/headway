import { describe, expect, test } from 'vitest';
import { LngLat } from 'maplibre-gl';
import Trip from './Trip';
import VehicleOverlay, { TrackCorrection } from './VehicleOverlay';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import {
  TransitVehicleMode,
  TravelmuxItinerary,
  TravelmuxLeg,
  TravelmuxMode,
} from 'src/services/TravelmuxClient';
import { DistanceUnits } from 'src/utils/models';

function trip(patternCode: string): Trip {
  const leg = {
    mode: TravelmuxMode.Transit,
    geometry: '',
    fromPlace: { location: [-122.3, 47.5], name: 'from' },
    toPlace: { location: [-122.3, 47.6], name: 'to' },
    startTime: '2024-05-17T12:35:01-07:00',
    endTime: '2024-05-17T12:45:01-07:00',
    distanceMeters: 5000,
    durationSeconds: 600,
    transitLeg: {
      vehicleMode: TransitVehicleMode.Bus,
      route: { shortName: '40' },
      patternCode,
      realTime: false,
      alerts: [],
    },
  } as TravelmuxLeg;
  const itinerary: TravelmuxItinerary = {
    mode: TravelmuxMode.Transit,
    startTime: '2024-05-17T12:35:01-07:00',
    endTime: '2024-05-17T13:05:01-07:00',
    durationSeconds: 1800,
    distanceMeters: 5000,
    bounds: { min: [-122.3, 47.5], max: [-122.3, 47.6] },
    legs: [leg],
  };
  return new Trip(itinerary, DistanceUnits.Kilometers);
}

function overlay(trips: Trip[]): VehicleOverlay {
  const point = new LngLat(-122.3, 47.5);
  return new VehicleOverlay({} as BaseMapInterface, point, point, trips);
}

describe('TrackCorrection', () => {
  test('places a newly appearing marker on its track', () => {
    const correction = new TrackCorrection();

    expect(correction.apply(new LngLat(-122.3, 47.5), 0).lng).toEqual(-122.3);
  });

  test('eases a refreshed track from the position already drawn', () => {
    const correction = new TrackCorrection();
    correction.begin(new LngLat(0, 0), 0);

    expect(correction.apply(new LngLat(10, 0), 500).lng).toEqual(5);
    expect(correction.apply(new LngLat(20, 0), 1000).lng).toEqual(20);
  });

  test('starts a second refresh from a correction already in progress', () => {
    const correction = new TrackCorrection();
    correction.begin(new LngLat(0, 0), 0);
    const current = correction.apply(new LngLat(10, 0), 500);
    correction.begin(current, 500);

    expect(correction.apply(new LngLat(20, 0), 500).lng).toEqual(5);
  });
});

describe('tripToSelect', () => {
  const routeForty = trip('1:40:0:01');
  const routeEight = trip('1:8:0:02');

  test('finds the trip running the clicked vehicle’s pattern', () => {
    const subject = overlay([routeForty, routeEight]);
    subject.selectTrip(routeForty);
    expect(subject.tripToSelect('1:8:0:02')).toBe(routeEight);
  });

  test('has nothing to select for a vehicle no trip on screen runs', () => {
    const subject = overlay([routeForty, routeEight]);
    subject.selectTrip(routeForty);
    expect(subject.tripToSelect('1:99:0:03')).toBeUndefined();
  });

  test('leaves the selection alone for a vehicle on the selected trip', () => {
    const subject = overlay([routeForty, routeEight]);
    subject.selectTrip(routeForty);
    expect(subject.tripToSelect('1:40:0:01')).toBeUndefined();
  });

  test('selects from a vehicle clicked before anything is picked', () => {
    const subject = overlay([routeForty, routeEight]);
    expect(subject.tripToSelect('1:40:0:01')).toBe(routeForty);
  });
});
