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
  return new TransitVehicle(raw, '#0080FF');
}

function reportedSecondsAgo(seconds: number): TransitVehicle {
  const lastUpdated = new Date(NOW.getTime() - seconds * 1000).toISOString();
  return vehicle({ lastUpdated });
}

describe('asOfFormatted', () => {
  test('a position from seconds ago', () => {
    expect(reportedSecondsAgo(40).asOfFormatted(NOW)).toEqual(
      'Location as of 40 sec ago',
    );
  });

  test('a position from minutes ago', () => {
    expect(reportedSecondsAgo(150).asOfFormatted(NOW)).toEqual(
      'Location as of 3 min ago',
    );
  });

  // Clocks disagree, and a vehicle reporting from the near future shouldn't read as "-5 sec ago".
  test('a position stamped in the future reads as current', () => {
    expect(reportedSecondsAgo(-5).asOfFormatted(NOW)).toEqual(
      'Location as of 0 sec ago',
    );
  });

  test('a position with no timestamp', () => {
    expect(vehicle().asOfFormatted(NOW)).toEqual('Live location');
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
