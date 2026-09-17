import { describe, expect, test } from 'vitest';
import { TransitVehicle } from './VehicleOverlay';
import { TravelmuxVehicle } from 'src/services/TravelmuxClient';

const NOW = new Date('2024-05-17T12:35:01-07:00');

function vehicle(overrides: Partial<TravelmuxVehicle> = {}): TransitVehicle {
  const raw: TravelmuxVehicle = {
    patternCode: '1:40:0:01',
    vehicleId: '1:7204',
    label: '7204',
    lat: 47.6,
    lon: -122.33,
    ...overrides,
  };
  return new TransitVehicle(raw, {
    color: '#0080FF',
    emoji: '🚍',
    routeName: '5',
  });
}

function reportedSecondsAgo(seconds: number): TransitVehicle {
  const lastUpdated = new Date(NOW.getTime() - seconds * 1000).toISOString();
  return vehicle({ lastUpdated });
}

describe('asOfFormatted', () => {
  test('a position from seconds ago', () => {
    expect(reportedSecondsAgo(40).freshnessFormatted(NOW)).toEqual(
      'Location as of 40 sec ago',
    );
  });

  test('a position from minutes ago', () => {
    expect(reportedSecondsAgo(150).freshnessFormatted(NOW)).toEqual(
      'Location as of 3 min ago',
    );
  });

  // Clocks disagree, and a vehicle reporting from the near future shouldn't read as "-5 sec ago".
  test('a position stamped in the future reads as current', () => {
    expect(reportedSecondsAgo(-5).freshnessFormatted(NOW)).toEqual(
      'Location as of 0 sec ago',
    );
  });

  test('a position with no timestamp', () => {
    expect(vehicle().freshnessFormatted(NOW)).toEqual('Live location');
  });
});

describe('positionAt', () => {
  const T0 = NOW.getTime();
  // 10s of track, running due east from the reported position.
  const tracked = () =>
    vehicle({
      lat: 47.6,
      lon: -122.33,
      lastUpdated: new Date(T0).toISOString(),
      track: [
        { lat: 47.6, lon: -122.33, time: new Date(T0).toISOString() },
        { lat: 47.6, lon: -122.32, time: new Date(T0 + 5000).toISOString() },
        { lat: 47.6, lon: -122.31, time: new Date(T0 + 10000).toISOString() },
      ],
    });

  test('a vehicle with no track sits where it reported', () => {
    const v = vehicle({ track: undefined });
    expect(v.positionAt(T0 + 60000).lng).toEqual(v.lngLat.lng);
  });

  test('at the start of the track it is where it reported', () => {
    expect(tracked().positionAt(T0).lng).toBeCloseTo(-122.33, 6);
  });

  test('halfway between two waypoints it is halfway between them', () => {
    expect(tracked().positionAt(T0 + 2500).lng).toBeCloseTo(-122.325, 6);
  });

  test('it lands on a waypoint at that waypoint`s time', () => {
    expect(tracked().positionAt(T0 + 5000).lng).toBeCloseTo(-122.32, 6);
  });

  // We only guess as far as travelmux predicted; past that we hold rather than fly off the end.
  test('past the end of the track it holds at the last waypoint', () => {
    expect(tracked().positionAt(T0 + 999999).lng).toBeCloseTo(-122.31, 6);
  });

  test('it only calls itself estimated once it has left the reported position', () => {
    expect(tracked().isEstimatedAt(T0)).toBe(false);
    expect(tracked().isEstimatedAt(T0 + 1)).toBe(true);
    expect(vehicle({ track: undefined }).isEstimatedAt(T0 + 60000)).toBe(false);
  });

  test('a dot that has moved on says so rather than claiming to be a report', () => {
    const v = tracked();
    expect(v.freshnessFormatted(new Date(T0 + 4000))).toEqual(
      'Estimated \u00b7 confirmed 4 sec ago',
    );
  });
});

describe('labelFormatted', () => {
  test('a bus with a fleet number', () => {
    expect(vehicle({ label: '7193' }).labelFormatted).toEqual('(vehicle 7193)');
  });

  // Link, Sounder and the ferries report a position but no label.
  test('a vehicle that publishes no label', () => {
    expect(vehicle({ label: undefined }).labelFormatted).toBeUndefined();
  });
});

describe('markerKey', () => {
  test('distinguishes vehicles serving the same pattern', () => {
    expect(vehicle({ vehicleId: '1:7204' }).markerKey).not.toEqual(
      vehicle({ vehicleId: '1:7205' }).markerKey,
    );
  });

  test('falls back to the label when a feed omits the vehicle id', () => {
    expect(vehicle({ vehicleId: undefined, label: '7204' }).markerKey).toEqual(
      'vehicle_1:40:0:01_7204',
    );
  });
});
