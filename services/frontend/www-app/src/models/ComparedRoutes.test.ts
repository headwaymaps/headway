import { expect, test } from 'vitest';
import ComparedRoutes from './ComparedRoutes';
import Trip from './Trip';
import { DistanceUnits } from 'src/utils/models';
import { TravelmuxMode } from 'src/services/TravelmuxClient';

function trip(geometry: string, durationSeconds: number): Trip {
  return new Trip(
    {
      mode: TravelmuxMode.Walk,
      startTime: '2024-05-17T12:35:01-07:00',
      endTime: '2024-05-17T13:05:01-07:00',
      durationSeconds,
      distanceMeters: 1000,
      bounds: { min: [-122.3, 47.5], max: [-122.3, 47.6] },
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

test('routers share only identical routes', () => {
  const compared = new ComparedRoutes();
  compared.add('before', [trip('abc', 600)]);
  compared.add('quicker', [trip('abc', 540)]);
  compared.add('elsewhere', [trip('xyz', 600)]);

  const [shared, elsewhere] = compared.routes;
  expect(shared!.plans.map((plan) => plan.router)).toEqual([
    'before',
    'quicker',
  ]);
  expect(shared!.plans[1]!.durationSeconds).toEqual(540);
  expect(elsewhere!.plans.map((plan) => plan.router)).toEqual(['elsewhere']);
  expect(shared!.trip.raw.routePreferences).toBeUndefined();
});

test('one router keeps its own routes along the same streets', () => {
  const compared = new ComparedRoutes();
  compared.add('only', [trip('abc', 600), trip('abc', 660)]);

  expect(compared.routes).toHaveLength(2);
});
