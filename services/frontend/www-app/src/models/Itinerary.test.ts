import { describe, expect, test } from 'vitest';
import Itinerary from './Itinerary';
import {
  OTPItinerary,
  OTPItineraryLeg,
  OTPMode,
} from 'src/services/OpenTripPlannerAPI';
import { DistanceUnits } from 'src/utils/models';

function leg(mode: OTPMode, distance: number): OTPItineraryLeg {
  return {
    startTime: 1715000000000,
    endTime: 1715000600000,
    mode,
    transitLeg: mode !== OTPMode.Walk && mode !== OTPMode.Bicycle,
    distance,
    legGeometry: { points: '' },
    realTime: false,
    from: { name: 'from', lat: 47.5, lon: -122.3 },
    to: { name: 'to', lat: 47.6, lon: -122.3 },
    alerts: [],
  };
}

function itinerary(legs: OTPItineraryLeg[], walkDistance: number): Itinerary {
  const raw: OTPItinerary = {
    generalizedCost: 0,
    duration: 1800,
    startTime: 1715000000000,
    endTime: 1715001800000,
    walkDistance,
    legs,
  };
  const withBicycle = legs.some((l) => l.mode === OTPMode.Bicycle);
  return Itinerary.fromOtp(raw, DistanceUnits.Kilometers, withBicycle);
}

describe('nonTransitDistance', () => {
  test('sums the walking legs of a transit itinerary', () => {
    const trip = itinerary(
      [
        leg(OTPMode.Walk, 696.33),
        leg(OTPMode.Bus, 5809.19),
        leg(OTPMode.Walk, 222.24),
      ],
      918.57,
    );
    expect(trip.nonTransitDistance).toBeCloseTo(918.57);
    expect(trip.walkingDistanceFormatted).toEqual('0.9 km walk total');
  });

  // OTP's own `walkDistance` is 0 here, since none of the legs involve walking.
  test('sums the cycling legs of a bike+transit itinerary', () => {
    const trip = itinerary(
      [
        leg(OTPMode.Bicycle, 2478.0),
        leg(OTPMode.Bus, 4003.0),
        leg(OTPMode.Bicycle, 1029.0),
      ],
      0,
    );
    expect(trip.nonTransitDistance).toBeCloseTo(3507.0);
    expect(trip.walkingDistanceFormatted).toEqual('3.5 km bike total');
  });
});
