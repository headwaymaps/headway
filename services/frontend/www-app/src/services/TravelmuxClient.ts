import { LngLat, LngLatLike } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Ok, Err, Result } from 'src/utils/Result';
import Trip, { TripFetchError } from 'src/models/Trip';
import { ValhallaErrorCode } from './ValhallaAPI';

export interface TravelmuxPlanResponse {
  itineraries: TravelmuxItinerary[];
}

export interface TravelmuxItinerary {
  mode: TravelmuxMode;
  /// RFC 3339, in the timezone of the graph that planned the trip
  startTime: string;
  /// RFC 3339, in the timezone of the graph that planned the trip
  endTime: string;
  durationSeconds: number;
  distanceMeters: number;
  bounds: { min: [number, number]; max: [number, number] };
  legs: TravelmuxLeg[];
}

export interface TravelmuxLeg {
  mode: TravelmuxMode;
  /// encoded polyline, 1e-6 scale
  geometry: string;
  fromPlace: TravelmuxPlace;
  toPlace: TravelmuxPlace;
  /// RFC 3339. Includes any real-time delay travelmux knows about.
  startTime: string;
  /// RFC 3339. Includes any real-time delay travelmux knows about.
  endTime: string;
  distanceMeters: number;
  durationSeconds: number;
  // Exactly one of these is set
  transitLeg?: TransitLeg;
  nonTransitLeg?: NonTransitLeg;
}

export interface TravelmuxPlace {
  lat: number;
  lon: number;
  /// Transit stops have names. Places the user picked usually don't.
  name?: string;
}

export interface TransitLeg {
  /// What kind of vehicle this is a ride on. The leg's own `mode` is always TRANSIT.
  vehicleMode: TransitVehicleMode;
  route?: TransitRoute;
  agencyName?: string;
  headsign?: string;
  /// Whether the leg's times reflect real-time data, rather than just the schedule
  realTime: boolean;
  alerts: TransitAlert[];
}

export interface TransitRoute {
  shortName?: string;
  longName?: string;
  /// An RRGGBB hex color, without a leading "#"
  color?: string;
}

export interface TransitAlert {
  headerText?: string;
  descriptionText: string;
  url?: string;
  /// RFC 3339
  effectiveStart?: string;
  /// RFC 3339
  effectiveEnd?: string;
}

export interface NonTransitLeg {
  maneuvers: [TravelmuxManeuver];
  substantialStreetNames?: string[];
}

export interface TravelmuxManeuver {
  instruction?: string;
  verbalPostTransitionInstruction?: string;
  startPoint: LngLatLike;
  bearingBefore: number;
  bearingAfter: number;
  // same as valhalla's maneuver type
  type: number;
}

// Non-exaustive
export enum TravelmuxErrorCode {
  // No transit graph covers the requested area, either because travelmux has no OTP instance
  // serving it or because OTP itself reported the trip as out of bounds.
  TransitUnsupportedArea = 1701,

  // Currently, errors originating in Valhalla are +2000
  ValhallaUnsupportedArea = ValhallaErrorCode.UnsupportedArea + 2000,
}

export interface TravelmuxError {
  errorCode: TravelmuxErrorCode;
  statusCode: number;
  message: string;
}

// incomplete
export type TravelmuxPlanRequest = {
  fromPlace: string;
  toPlace: string;
  // It'd be nice to typecheck this as numeric, but it would require some
  // additional type juggling elsewhere
  //numItineraries?: number,
  numItineraries?: string;
  /// An RFC 3339 instant, or a local wall clock time like "2024-06-13T14:30", which travelmux
  /// interprets in the timezone of the graph serving the trip.
  dateTime?: string;
  arriveBy?: string;
  // comma separated list Mode(s)
  mode?: string;
  /// Only affects the prose of an instruction ("Continue for 2 miles.") - every distance in the
  /// response is in meters.
  preferredDistanceUnits: string;
};

export enum TravelmuxMode {
  Bike = 'BICYCLE',
  Walk = 'WALK',
  Drive = 'CAR',
  Transit = 'TRANSIT',
}

/// The kind of vehicle a transit leg is a ride on, as OTP names it.
///
/// Non-exhaustive: OTP has more of these (COACH, MONORAIL, TROLLEYBUS, ...) and travelmux passes
/// them through verbatim, so treat an unrecognized value as generic transit.
export enum TransitVehicleMode {
  Bus = 'BUS',
  CableCar = 'CABLE_CAR',
  Ferry = 'FERRY',
  Funicular = 'FUNICULAR',
  Gondola = 'GONDOLA',
  Rail = 'RAIL',
  Subway = 'SUBWAY',
  Tram = 'TRAM',
  Transit = 'TRANSIT',
}

export interface ElevationResponse {
  sampledGeometry: string;
  elevation: number[];
  totalClimbMeters: number;
  totalFallMeters: number;
}

export class TravelmuxClient {
  public static async fetchElevation(
    path: string,
  ): Promise<Result<ElevationResponse, Error>> {
    const params = new URLSearchParams({ path });
    const response = await fetch(`/travelmux/v7/elevation?${params}`);

    if (response.ok) {
      const elevationData: ElevationResponse = await response.json();
      return Ok(elevationData);
    } else {
      const error = new Error(
        `Failed to fetch elevation: ${response.statusText}`,
      );
      return Err(error);
    }
  }

  public static async fetchPlans(
    from: LngLat,
    to: LngLat,
    modes: TravelmuxMode[],
    numItineraries: number,
    preferredDistanceUnits: DistanceUnits,
    time?: string,
    date?: string,
    arriveBy?: boolean,
  ): Promise<Result<Trip[], TripFetchError>> {
    const params: TravelmuxPlanRequest = {
      fromPlace: `${from.lat},${from.lng}`,
      toPlace: `${to.lat},${to.lng}`,
      numItineraries: `${numItineraries}`,
      mode: modes.join(','),
      preferredDistanceUnits,
    };

    // travelmux plans from "now" unless we name a departure (or arrival) time. The time the user
    // picked is a wall clock time where they're traveling, not necessarily where they are, so we
    // send it without an offset and let travelmux resolve it in the graph's timezone.
    if (date) {
      params['dateTime'] = `${date}T${time ?? '00:00'}`;
    } else {
      console.assert(
        !time,
        'travelmux requires that if time is specified, date must also be specified',
      );
    }
    if (arriveBy) {
      params['arriveBy'] = true.toString();
    }

    const query = new URLSearchParams(params).toString();

    const response = await fetch('/travelmux/v7/plan?' + query);

    if (response.ok) {
      const plan: TravelmuxPlanResponse = await response.json();
      const trips = plan.itineraries.map(
        (itinerary: TravelmuxItinerary) =>
          new Trip(itinerary, preferredDistanceUnits),
      );
      return Ok(trips);
    } else {
      const errorBody = await response.json();
      const error = errorBody['error'];
      console.assert(error);
      const routeError = TripFetchError.fromTravelmux(error);
      return Err(routeError);
    }
  }
}

export function travelModeFromTravelmuxMode(mode: TravelmuxMode): TravelMode {
  switch (mode) {
    case TravelmuxMode.Walk:
      return TravelMode.Walk;
    case TravelmuxMode.Bike:
      return TravelMode.Bike;
    case TravelmuxMode.Drive:
      return TravelMode.Drive;
    case TravelmuxMode.Transit:
      return TravelMode.Transit;
  }
}
