import { describe, expect, test } from 'vitest';
import Trip from './Trip';
import TripGroup from './TripGroup';
import {
  LonLatPair,
  TransitVehicleMode,
  TravelmuxItinerary,
  TravelmuxLeg,
  TravelmuxMode,
} from 'src/services/TravelmuxClient';
import { DistanceUnits } from 'src/utils/models';

const uDistrict: LonLatPair = [-122.31406, 47.66023];
const symphony: LonLatPair = [-122.33623, 47.60803];

function walk(mode = TravelmuxMode.Walk): TravelmuxLeg {
  return {
    mode,
    geometry: '',
    fromPlace: { location: [-122.3, 47.6] },
    toPlace: { location: [-122.3, 47.6] },
    startTime: '2026-11-19T15:00:00-08:00',
    endTime: '2026-11-19T15:05:00-08:00',
    distanceMeters: 300,
    durationSeconds: 300,
    nonTransitLeg: { maneuvers: [], substantialStreetNames: [] },
  } as unknown as TravelmuxLeg;
}

const routeColors: Record<string, [string, string]> = {
  '1 Line': ['28813F', 'FFFFFF'],
  '2 Line': ['007CAD', 'FFFFFF'],
  '36': ['FDB71A', '000000'],
};

function ride(
  route: string,
  from: LonLatPair,
  to: LonLatPair,
  departs: string,
): TravelmuxLeg {
  return {
    mode: TravelmuxMode.Transit,
    geometry: '',
    fromPlace: { location: from, name: 'from' },
    toPlace: { location: to, name: 'to' },
    startTime: `2026-11-19T${departs}:00-08:00`,
    endTime: `2026-11-19T${departs}:00-08:00`,
    distanceMeters: 5000,
    durationSeconds: 600,
    transitLeg: {
      vehicleMode: TransitVehicleMode.Subway,
      route: {
        shortName: route,
        color: routeColors[route]![0],
        textColor: routeColors[route]![1],
      },
      realTime: false,
      alerts: [],
    },
  };
}

/// `tripKey` is the key travelmux would send, which it omits when there's no transit leg.
function trip(
  legs: TravelmuxLeg[],
  tripKey?: string,
  mode = TravelmuxMode.Transit,
): Trip {
  const itinerary: TravelmuxItinerary = {
    mode,
    startTime: legs.find((leg) => leg.transitLeg)?.startTime ?? '',
    endTime: legs[legs.length - 1]!.endTime,
    durationSeconds: 1080,
    distanceMeters: 5600,
    bounds: { min: [-122.4, 47.6], max: [-122.3, 47.7] },
    routerId: 'otp',
    legs,
    tripKey,
  };
  return new Trip(itinerary, DistanceUnits.Miles);
}

function lightRail(route: string, departs: string, tripKey = 'rail'): Trip {
  return trip(
    [walk(), ride(route, uDistrict, symphony, departs), walk()],
    tripKey,
  );
}

describe('TripGroup.group', () => {
  test('lines serving the same stops are one trip, in departure order', () => {
    const at1506 = lightRail('1 Line', '15:06');
    // The 2 Line leaves first, but still badges second.
    const at1456 = lightRail('2 Line', '14:56');
    const at1501 = lightRail('1 Line', '15:01');
    const groups = TripGroup.group([at1506, at1456, at1501]);

    expect(groups).toHaveLength(1);
    expect(groups[0]!.trips).toEqual([at1456, at1501, at1506]);
    const [walkThere, rail, walkAway] = groups[0]!.legBadges;
    expect(walkThere).toEqual({
      kind: 'selfPowered',
      emoji: '🚶‍♀️',
      minutes: 5,
    });
    expect(rail).toEqual({
      kind: 'ride',
      emoji: '🚇',
      routes: [
        { label: '1 Line', color: '#28813F', textColor: '#FFFFFF' },
        { label: '2 Line', color: '#007CAD', textColor: '#FFFFFF' },
      ],
    });
    expect(walkAway!.kind).toEqual('selfPowered');
  });

  test('groups keep the order their first trip came in', () => {
    const rail = lightRail('1 Line', '14:59');
    const bus = trip(
      [walk(), ride('36', [-122.29, 47.53], symphony, '15:01'), walk()],
      'bus',
    );
    const railAgain = lightRail('1 Line', '15:07');
    const groups = TripGroup.group([rail, bus, railAgain]);
    expect(groups.map((g) => g.first)).toEqual([rail, bus]);
    expect(groups[1]!.legBadges[1]).toEqual({
      kind: 'ride',
      emoji: '🚇',
      routes: [{ label: '36', color: '#FDB71A', textColor: '#000000' }],
    });
  });

  test("other modes' alternatives are never merged", () => {
    const drive = () =>
      trip([walk(TravelmuxMode.Drive)], undefined, TravelmuxMode.Drive);
    expect(TripGroup.group([drive(), drive()])).toHaveLength(2);
  });

  test('a different key is a different trip', () => {
    expect(
      TripGroup.group([
        lightRail('1 Line', '15:01', 'a'),
        lightRail('1 Line', '15:11', 'b'),
      ]),
    ).toHaveLength(2);
  });
});

describe('nextDeparture', () => {
  const [group] = TripGroup.group([
    lightRail('1 Line', '15:01'),
    lightRail('1 Line', '15:11'),
  ]);
  const at = (time: string) => new Date(`2026-11-19T${time}:00-08:00`);

  test('the first yet to leave', () => {
    expect(group!.nextDeparture(at('15:00'))).toBe(group!.trips[0]);
    expect(group!.nextDeparture(at('15:05'))).toBe(group!.trips[1]);
  });

  test('the last, once they all have left', () => {
    expect(group!.nextDeparture(at('15:20'))).toBe(group!.trips[1]);
  });
});
