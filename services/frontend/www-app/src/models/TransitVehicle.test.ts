import { describe, expect, test } from 'vitest';
import TransitVehicle from './TransitVehicle';
import {
  TransitVehicleMode,
  TravelmuxVehicle,
} from 'src/services/TravelmuxClient';

const NOW = new Date('2024-05-17T12:35:01-07:00');

function vehicle(overrides: Partial<TravelmuxVehicle> = {}): TransitVehicle {
  const raw: TravelmuxVehicle = {
    id: '1:40:0:01/1:7204',
    patternCode: '1:40:0:01',
    route: { shortName: '5', color: '0080FF' },
    vehicleMode: TransitVehicleMode.Bus,
    vehicleId: '1:7204',
    label: '7204',
    position: [-122.33, 47.6],
    lastUpdated: NOW.toISOString(),
    ...overrides,
  };
  return new TransitVehicle(raw);
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
});

describe('positionAt', () => {
  const T0 = NOW.getTime();
  // 10s of track, running due east from the reported position.
  const tracked = () =>
    vehicle({
      position: [-122.33, 47.6],
      lastUpdated: new Date(T0).toISOString(),
      track: {
        stepSeconds: 5,
        points: [
          [-122.33, 47.6],
          [-122.32, 47.6],
          [-122.31, 47.6],
        ],
      },
    });

  test('a vehicle with no track sits where it reported', () => {
    const v = vehicle({ track: undefined });
    expect(v.positionAt(T0 + 60000).lng).toEqual(v.lngLat.lng);
  });

  // The samples are evenly spaced, so the bracketing pair is an index rather than a search.
  test('it indexes into the track by elapsed time', () => {
    expect(tracked().positionAt(T0 + 7500).lng).toBeCloseTo(-122.315, 6);
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

describe('boardingStopFormatted', () => {
  function arrivingIn(seconds: number, state: 'approaching' | 'departed') {
    const arrival = new Date(NOW.getTime() + seconds * 1000).toISOString();
    return vehicle({ boardingStop: { state, arrival } });
  }

  test('a vehicle still on its way counts down to the stop', () => {
    expect(arrivingIn(180, 'approaching').boardingStopFormatted(NOW)).toEqual(
      '3 min away',
    );
  });

  // A countdown of seconds is no use to someone who should be looking up the street.
  test('a vehicle about to arrive says so', () => {
    expect(arrivingIn(20, 'approaching').boardingStopFormatted(NOW)).toEqual(
      'Arriving now',
    );
  });

  test('a vehicle that has left the stop says how long ago', () => {
    expect(arrivingIn(-120, 'departed').boardingStopFormatted(NOW)).toEqual(
      'Left 2 min ago',
    );
  });

  // Travelmux decides which side of the stop a vehicle is on, so a late one can be past the stop
  // with an arrival still a few seconds out.
  test('a vehicle just past the stop does not count backwards', () => {
    expect(arrivingIn(5, 'departed').boardingStopFormatted(NOW)).toEqual(
      'Left 0 sec ago',
    );
  });

  test('a vehicle travelmux said nothing about', () => {
    expect(
      vehicle({ boardingStop: undefined }).boardingStopFormatted(NOW),
    ).toBeUndefined();
  });
});

describe('stopsAwayFormatted', () => {
  function approaching(stopsAway?: number): TransitVehicle {
    return vehicle({
      boardingStop: {
        state: 'approaching',
        arrival: NOW.toISOString(),
        stopsAway,
      },
    });
  }

  test('a vehicle a few stops up the route', () => {
    expect(approaching(3).stopsAwayFormatted).toEqual('3 stops away');
  });

  test('a vehicle working towards the rider`s own stop', () => {
    expect(approaching(1).stopsAwayFormatted).toEqual('next stop');
  });

  test('a vehicle whose feed won`t say which stop it is working towards', () => {
    expect(approaching(undefined).stopsAwayFormatted).toBeUndefined();
  });

  // Nothing to wait through once it has been and gone.
  test('a vehicle that has left the stop', () => {
    const departed = vehicle({
      boardingStop: { state: 'departed', arrival: NOW.toISOString() },
    });
    expect(departed.stopsAwayFormatted).toBeUndefined();
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
  // Identity is travelmux's to decide - it knows a vehicle reports under one id for as long as
  // it's on a pattern. The client just needs it to be stable between polls.
  test('follows the id travelmux gave the vehicle', () => {
    expect(vehicle({ id: '1:40:0:01/1:7204' }).markerKey).not.toEqual(
      vehicle({ id: '1:40:0:01/1:7205' }).markerKey,
    );
    expect(vehicle({ id: '1:40:0:01/1:7204' }).markerKey).toEqual(
      vehicle({ id: '1:40:0:01/1:7204' }).markerKey,
    );
  });
});
