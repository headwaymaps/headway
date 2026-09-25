import { LngLat, LngLatLike } from 'maplibre-gl';

/// Every point travelmux writes, in requests and responses alike.
export type LonLatPair = [lon: number, lat: number];
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
  /// `[lon, lat]`
  location: LonLatPair;
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
  /// The pattern this ride follows, which is what vehicle positions are keyed by. Only meaningful
  /// for the life of the plan it came in - OTP renumbers patterns when transit data is rebuilt.
  patternCode?: string;
  /// The whole shape the pattern runs, as an encoded polyline, 1e-6 scale. The leg's own geometry
  /// is the slice of this the rider is aboard for.
  patternGeometry?: string;
  /// Every ordinary stop on the portion of the pattern the rider travels, in order.
  riddenStops?: string;
  /// The stops beyond the ridden portion, packed the same way.
  contextStops?: string;
  /// Where the rider boards and alights, packed the same way.
  onOffStops?: string;
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

export interface TravelmuxVehiclePositionsResponse {
  /// RFC 3339. What the clock said on the server as this was answered.
  serverTime: string;
  /// Requested patterns the graph has never heard of - codes that died when the transit data was
  /// rebuilt, as opposed to a route that simply isn't running.
  unknownPatterns?: string[];
  vehicles: TravelmuxVehicle[];
}

/// Vehicles, and how far this device's clock is from the one that timed them.
export interface VehiclePositions {
  vehicles: TravelmuxVehicle[];
  /// Milliseconds to add to this device's clock to read the server's.
  clockOffsetMs: number;
}

/// One transit vehicle's last known position.
export interface TravelmuxVehicle {
  /// Stable for as long as the vehicle keeps reporting on this pattern.
  id: string;
  /// Which pattern this vehicle is serving, matching a transit leg's patternCode.
  patternCode: string;
  /// What the vehicle is running. Carried on the vehicle so a client needn't join back to the
  /// plan's legs to find out.
  route: TransitRoute;
  /// What kind of vehicle it is, as OTP names it.
  vehicleMode?: TransitVehicleMode;
  headsign?: string;
  /// `FeedId:VehicleId`
  vehicleId?: string;
  /// What the vehicle shows the public, e.g. a fleet number
  label?: string;
  /// Where the vehicle last reported being, as `[lon, lat]`.
  position: LonLatPair;
  /// Degrees clockwise from north, as the feed reported it. Most feeds don't publish one.
  heading?: number;
  /// RFC 3339. When the vehicle reported this position. Travelmux drops a vehicle that can't
  /// date its report, so this is always here.
  lastUpdated: string;
  /// Where travelmux guesses the vehicle goes next, to animate along between polls.
  track?: TravelmuxTrack;
  /// When this vehicle is at the rider's boarding stop. Absent when no boarding stop was asked
  /// about, or when the vehicle's trip doesn't call there.
  boardingStop?: TravelmuxBoardingStop;
}

/// When a vehicle is at the rider's boarding stop, on whichever side of it the vehicle is.
///
/// `arrival` is RFC 3339, not a countdown: a poll is held for 30 seconds, and a number of minutes
/// would be that stale by the end of one.
export type TravelmuxBoardingStop =
  | {
      state: 'approaching';
      arrival: string;
      /// RFC 3339. When the vehicle is at each stop from the one it's working towards through
      /// the rider's own, in order - the same predictions its track is paced by, so counting the
      /// ones still ahead counts the stops the animated vehicle has yet to make.
      stopArrivals?: string[];
    }
  | { state: 'departed'; arrival: string };

/// A pattern to report vehicles for, and where the rider boards it.
export interface PatternRequest {
  code: string;
  boardingStop?: LngLatLike;
}

/// Positions at a fixed cadence, beginning at the vehicle's own `lastUpdated`: `points[0]` is
/// where it was when it reported, and everything after is a guess. Evenly spaced in time, so the
/// pair bracketing an instant is arithmetic rather than a search.
export interface TravelmuxTrack {
  stepSeconds: number;
  /// At least two `[lon, lat]` points. The first is `lastUpdated`'s.
  points: LonLatPair[];
}

export interface NonTransitLeg {
  maneuvers: [TravelmuxManeuver];
  substantialStreetNames?: string[];
}

export interface TravelmuxManeuver {
  instruction?: string;
  verbalPostTransitionInstruction?: string;
  startPoint: LonLatPair;
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

/// v8 writes every point as a `[lon, lat]` pair, in requests as well as responses.
function lonLatPair(point: LngLatLike): LonLatPair {
  const { lng, lat } = LngLat.convert(point);
  return [lng, lat];
}

export class TravelmuxClient {
  public static async fetchElevation(
    path: string,
  ): Promise<Result<ElevationResponse, Error>> {
    const params = new URLSearchParams({ path });
    const response = await fetch(`/travelmux/v8/elevation?${params}`);

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

  /// Where the vehicles near each requested pattern's boarding stop are right now.
  ///
  /// `from`/`to` are the endpoints of the plan the patterns came from: pattern codes only mean
  /// something to the transit graph that issued them, so they pick the same one. Each pattern
  /// names where its leg boards, which is what travelmux measures "near" against - a pattern
  /// runs its whole length, and most of its vehicles have nothing to do with the trip.
  public static async fetchVehiclePositions(
    from: LngLat,
    to: LngLat,
    patterns: PatternRequest[],
  ): Promise<Result<VehiclePositions, Error>> {
    if (patterns.length === 0) {
      return Ok({ vehicles: [], clockOffsetMs: 0 });
    }

    const body = {
      fromPlace: lonLatPair(from),
      toPlace: lonLatPair(to),
      patterns: patterns.map(({ code, boardingStop }) => ({
        code,
        boardingStop: boardingStop && lonLatPair(boardingStop),
      })),
    };

    // A dropped connection or a body that isn't JSON throws rather than returning, and this is
    // called from a timer with nowhere for a rejection to go - so it becomes an Err like any
    // other failed poll.
    try {
      const response = await fetch('/travelmux/v8/vehicle_positions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        return Err(
          new Error(
            `Failed to fetch vehicle positions: ${response.statusText}`,
          ),
        );
      }
      const parsed: TravelmuxVehiclePositionsResponse = await response.json();
      // Ignores the latency of the response itself, which is small next to the clock skew this
      // is here to correct - and erring towards "slightly stale" beats erring towards a guess.
      return Ok({
        vehicles: parsed.vehicles,
        clockOffsetMs: Date.parse(parsed.serverTime) - Date.now(),
      });
    } catch (e) {
      return Err(e instanceof Error ? e : new Error(String(e)));
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
      fromPlace: `${from.lng},${from.lat}`,
      toPlace: `${to.lng},${to.lat}`,
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

    const response = await fetch('/travelmux/v8/plan?' + query);

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
