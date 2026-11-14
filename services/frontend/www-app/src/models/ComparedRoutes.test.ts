import { expect, test } from 'vitest';
import ComparedRoutes from './ComparedRoutes';
import Trip from './Trip';
import { DistanceUnits } from 'src/utils/models';
import { TravelmuxMode } from 'src/services/TravelmuxClient';

function trip(
  geometry: string,
  durationSeconds: number,
  routerId: string,
): Trip {
  return new Trip(
    {
      mode: TravelmuxMode.Walk,
      startTime: '2024-05-17T12:35:01-07:00',
      endTime: '2024-05-17T13:05:01-07:00',
      durationSeconds,
      distanceMeters: 1000,
      bounds: { min: [-122.3, 47.5], max: [-122.3, 47.6] },
      routerId,
      legs: [
        {
          mode: TravelmuxMode.Walk,
          geometry,
          fromPlace: { location: [-122.3, 47.5], name: 'from' },
          toPlace: { location: [-122.3, 47.6], name: 'to' },
          startTime: '2024-05-17T12:35:01-07:00',
          endTime: '2024-05-17T13:05:01-07:00',
          distanceMeters: 1000,
          durationSeconds,
        },
      ],
      routePreferences: ['quieter'],
    },
    DistanceUnits.Kilometers,
  );
}

test('each route credits every router that took it', () => {
  const trips = [
    trip('abc', 600, 'otp'),
    trip('xyz', 600, 'otp'),
    trip('abd', 540, 'valhalla'),
  ];
  const { routes } = new ComparedRoutes(
    trips,
    {
      common: [],
      routes: [
        { itineraries: [0, 2], solo: [], partlyShared: [] },
        { itineraries: [1], solo: [], partlyShared: [] },
      ],
    },
    (routerId) => routerId.toUpperCase(),
  );

  const [shared, alone] = routes;
  expect(shared!.plans.map((plan) => plan.router)).toEqual(['OTP', 'VALHALLA']);
  expect(shared!.plans[1]!.durationSeconds).toEqual(540);
  expect(shared!.plans[1]!.routePreferences).toEqual(['quieter']);
  expect(shared!.trip.raw.routePreferences).toBeUndefined();
  expect(alone!.plans.map((plan) => plan.router)).toEqual(['OTP']);
});
