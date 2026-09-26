import {
  CircleLayerSpecification,
  LineLayerSpecification,
  LngLat,
  LngLatBounds,
} from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Result } from 'src/utils/Result';
import {
  TravelmuxMode,
  TravelmuxClient,
  TravelmuxItinerary,
  TravelmuxLeg,
  TransitAlert,
  TransitVehicleMode,
  travelModeFromTravelmuxMode,
  TravelmuxError,
  TravelmuxErrorCode,
} from 'src/services/TravelmuxClient';
import { formatDistance, formatDuration, formatTime } from 'src/utils/format';
import { decodePolyline } from 'src/utils/decodePolyline';
import { i18n } from 'src/i18n/lang';

/// The emoji standing in for a kind of transit vehicle.
export function transitVehicleEmoji(mode: TransitVehicleMode): string {
  switch (mode) {
    case TransitVehicleMode.Rail:
      return '🚆';
    case TransitVehicleMode.Subway:
      return '🚇';
    case TransitVehicleMode.CableCar:
    case TransitVehicleMode.Tram:
      return '🚊';
    case TransitVehicleMode.Funicular:
      return '🚡';
    case TransitVehicleMode.Gondola:
      return '🚠';
    case TransitVehicleMode.Ferry:
      return '⛴️';
    default:
      // BUS, TRANSIT, and anything else OTP might name
      return '🚍';
  }
}

export default class Trip {
  raw: TravelmuxItinerary;
  preferredDistanceUnits: DistanceUnits;
  legs: TripLeg[];

  constructor(raw: TravelmuxItinerary, preferredDistanceUnits: DistanceUnits) {
    this.raw = raw;
    this.preferredDistanceUnits = preferredDistanceUnits;
    this.legs = raw.legs.map((raw: TravelmuxLeg) => new TripLeg(raw));
  }

  get durationFormatted(): string {
    return formatDuration(this.raw.durationSeconds, 'shortform');
  }

  get viaRoadsFormatted(): string | null {
    const names = this.raw.legs.flatMap((leg) => {
      return leg.nonTransitLeg?.substantialStreetNames;
    });
    if (names.length == 0) {
      return null;
    }
    return names.join(i18n.global.t('punctuation_list_seperator'));
  }

  get distanceFormatted(): string {
    return formatDistance(
      this.raw.distanceMeters,
      DistanceUnits.Meters,
      this.preferredDistanceUnits,
    );
  }

  get bounds(): LngLatBounds {
    return new LngLatBounds(this.raw.bounds.min, this.raw.bounds.max);
  }

  get mode(): TravelMode {
    return travelModeFromTravelmuxMode(this.raw.mode);
  }

  get startTime(): Date {
    return new Date(this.raw.startTime);
  }

  get endTime(): Date {
    return new Date(this.raw.endTime);
  }

  get startStopTimesFormatted(): string {
    return i18n.global.t('time_range$startTime$endTime', {
      startTime: formatTime(this.startTime),
      endTime: formatTime(this.endTime),
    });
  }

  // How far the rider travels under their own power. Usually walking, but will be biking if mode
  // is transit+bike.
  get nonTransitDistanceMeters(): number {
    return this.legs
      .filter((leg) => !leg.transitLeg)
      .reduce((total, leg) => total + leg.distanceMeters, 0);
  }

  /// Whether the rider brings a bicycle along on their transit trip
  get withBicycle(): boolean {
    return this.legs.some((leg) => leg.raw.mode == TravelmuxMode.Bike);
  }

  get walkingDistanceFormatted(): string {
    const preformattedDistance = formatDistance(
      this.nonTransitDistanceMeters,
      DistanceUnits.Meters,
      this.preferredDistanceUnits,
    );

    if (this.withBicycle) {
      return i18n.global.t('bike_distance', { preformattedDistance });
    } else {
      return i18n.global.t('walk_distance', { preformattedDistance });
    }
  }

  get alerts(): TransitAlert[] {
    return this.legs.flatMap((leg) => leg.alerts);
  }

  get hasAlerts(): boolean {
    return this.alerts.length > 0;
  }

  /// Agencies often publish several unrelated alerts under one boilerplate header (BART sends
  /// everything as "BART.gov Alert"), and a trip can ride the same route twice, so we group by
  /// header and drop exact repeats. The details live in each alert's descriptionText.
  get alertGroups(): TransitAlertGroup[] {
    const groups: TransitAlertGroup[] = [];
    const groupsByHeader: Map<string, TransitAlertGroup> = new Map();

    for (const alert of this.alerts) {
      let group;
      if (alert.headerText === undefined) {
        // An alert with no header has nothing to group on, so it stands alone.
        group = { headerText: undefined, alerts: [] as TransitAlert[] };
        groups.push(group);
      } else {
        group = groupsByHeader.get(alert.headerText);
        if (!group) {
          group = {
            headerText: alert.headerText,
            alerts: [] as TransitAlert[],
          };
          groupsByHeader.set(alert.headerText, group);
          groups.push(group);
        }
      }

      const isRepeat = group.alerts.some(
        (existing) => existing.descriptionText === alert.descriptionText,
      );
      if (!isRepeat) {
        group.alerts.push(alert);
      }
    }

    return groups;
  }

  /// Whether the change into `legIdx` happens at a transit stop, which the map already marks
  /// with a stop of its own.
  transfersAtStop(legIdx: number): boolean {
    const leg = this.legs[legIdx];
    const previous = this.legs[legIdx - 1];
    return !!leg?.transitLeg || !!previous?.transitLeg;
  }

  /// The patterns this trip's transit legs ride, which is what live vehicles are keyed by.
  get patternCodes(): string[] {
    return this.legs.flatMap((leg) => leg.raw.transitLeg?.patternCode ?? []);
  }

  get firstTransitLeg(): TripLeg | undefined {
    return this.legs.slice(0, 2).find((leg) => leg.transitLeg);
  }
}

/// Alerts sharing a header, presented as one collapsible row.
export interface TransitAlertGroup {
  headerText?: string;
  alerts: TransitAlert[];
}

/// The whole route behind a transit leg, and how to draw it.
export interface TripLegContextLayer {
  geometry: GeoJSON.LineString;
  paint: LineLayerSpecification['paint'];
}

export interface TripLegStopsLayer {
  geometry: GeoJSON.MultiPoint;
  paint: CircleLayerSpecification['paint'];
}

export class TripLeg {
  readonly raw: TravelmuxLeg;
  geometry: GeoJSON.LineString;
  /// The whole route this leg rides part of, for transit legs the server has a shape for.
  patternGeometry?: GeoJSON.LineString;
  /// Every ordinary stop on the portion of the route the rider travels, in order.
  riddenStops?: GeoJSON.MultiPoint;
  /// The stops beyond the part the rider is aboard for.
  contextStops?: GeoJSON.MultiPoint;
  /// The stops where the rider boards and alights, drawn onto the route.
  onOffStops?: GeoJSON.MultiPoint;

  constructor(raw: TravelmuxLeg) {
    this.raw = raw;
    const points = decodePolyline(this.raw.geometry, 6);
    this.geometry = {
      type: 'LineString',
      coordinates: points,
    };
    const patternGeometry = this.raw.transitLeg?.patternGeometry;
    if (patternGeometry) {
      this.patternGeometry = {
        type: 'LineString',
        coordinates: decodePolyline(patternGeometry, 6),
      };
    }
    // Travelmux packs the stops as a polyline too - it encodes a list of coordinates as well as
    // it encodes a shape.
    const riddenStops = this.raw.transitLeg?.riddenStops;
    if (riddenStops) {
      this.riddenStops = {
        type: 'MultiPoint',
        coordinates: decodePolyline(riddenStops, 6),
      };
    }
    const contextStops = this.raw.transitLeg?.contextStops;
    if (contextStops) {
      this.contextStops = {
        type: 'MultiPoint',
        coordinates: decodePolyline(contextStops, 6),
      };
    }
    const onOffStops = this.raw.transitLeg?.onOffStops;
    if (onOffStops) {
      this.onOffStops = {
        type: 'MultiPoint',
        coordinates: decodePolyline(onOffStops, 6),
      };
    }
  }

  get start(): LngLat {
    const lngLat = this.geometry.coordinates[0]!;
    return new LngLat(lngLat[0]!, lngLat[1]!);
  }

  get mode(): TravelMode {
    return travelModeFromTravelmuxMode(this.raw.mode);
  }

  get startTime(): Date {
    return new Date(this.raw.startTime);
  }

  get endTime(): Date {
    return new Date(this.raw.endTime);
  }

  get durationSeconds(): number {
    return this.raw.durationSeconds;
  }

  /// How far this leg travels, in meters
  get distanceMeters(): number {
    return this.raw.distanceMeters;
  }

  /// Whether this leg is a ride on a transit vehicle, as opposed to walking or cycling to one
  get transitLeg(): boolean {
    return this.raw.transitLeg !== undefined;
  }

  /// Whether this leg's times reflect real-time data, rather than just the schedule
  get realTime(): boolean {
    return this.raw.transitLeg?.realTime ?? false;
  }

  get alerts(): TransitAlert[] {
    return this.raw.transitLeg?.alerts ?? [];
  }

  get emoji(): string {
    const vehicleMode = this.raw.transitLeg?.vehicleMode;
    if (vehicleMode) {
      return transitVehicleEmoji(vehicleMode);
    }
    // not a transit leg - the traveler gets there themselves
    switch (this.raw.mode) {
      case TravelmuxMode.Bike:
        return '🚲';
      case TravelmuxMode.Drive:
        return '🚙';
      default:
        return '🚶‍♀️';
    }
  }

  get shortName(): string {
    const route = this.raw.transitLeg?.route;
    const shortName = route?.shortName ?? route?.longName ?? '';
    return `${this.emoji} ${shortName}`.trim();
  }

  get sourceName(): string {
    return this.raw.fromPlace.name ?? '';
  }

  get destinationName(): string {
    return this.raw.toPlace.name ?? '';
  }

  get sourceLngLat(): LngLat {
    return new LngLat(...this.raw.fromPlace.location);
  }

  get destinationLngLat(): LngLat {
    return new LngLat(...this.raw.toPlace.location);
  }

  get departureLocationName(): string | undefined {
    return this.raw.fromPlace.name;
  }

  /// The dimmed line for the rest of the route, drawn under an active transit leg. Absent for a
  /// leg the server gave no pattern shape for.
  contextLayer(): TripLegContextLayer | undefined {
    const geometry = this.patternGeometry;
    if (!geometry) {
      return undefined;
    }
    return { geometry, paint: LineStyles.context(this.routeColor) };
  }

  /// A dot at each of the route's stops, so the rider can count what's between a vehicle and
  /// their own stop. Absent for a leg the server gave no stops for.
  riddenStopsLayer(): TripLegStopsLayer | undefined {
    const geometry = this.riddenStops;
    if (!geometry) {
      return undefined;
    }
    return { geometry, paint: CircleStyles.stop(this.routeColor) };
  }

  /// The stops beyond the ridden portion, faded like the line they sit on.
  contextStopsLayer(): TripLegStopsLayer | undefined {
    const geometry = this.contextStops;
    if (!geometry) {
      return undefined;
    }
    return { geometry, paint: CircleStyles.contextStop(this.routeColor) };
  }

  /// The stops where the rider boards and alights, drawn heavier than the ordinary route stops.
  onOffStopsLayer(): TripLegStopsLayer | undefined {
    const geometry = this.onOffStops;
    if (!geometry) {
      return undefined;
    }
    return { geometry, paint: CircleStyles.usedStop(this.routeColor) };
  }

  /// The route's own color, or the active line's where the feed doesn't name one.
  private get routeColor(): string {
    const routeColor = this.raw.transitLeg?.route?.color;
    return routeColor ? `#${routeColor}` : LineStyles.active['line-color'];
  }

  paintStyle(active: boolean): LineLayerSpecification['paint'] {
    if (active) {
      if (this.mode == TravelMode.Walk || this.mode == TravelMode.Bike) {
        return LineStyles.walkingActive;
      } else {
        const routeColor = this.raw.transitLeg?.route?.color;
        if (routeColor) {
          return LineStyles.activeColored(`#${routeColor}`);
        } else {
          return LineStyles.active;
        }
      }
    } else {
      if (this.mode == TravelMode.Walk || this.mode == TravelMode.Bike) {
        return LineStyles.walkingInactive;
      } else {
        return LineStyles.inactive;
      }
    }
  }
}

export enum TripFetchErrorCode {
  Other,
  UnsupportedNonTransitArea,
  UnsupportedTransitArea,
}

export class TripFetchError {
  errorCode: TripFetchErrorCode;
  message: string;

  constructor(errorCode: TripFetchErrorCode, message: string) {
    this.errorCode = errorCode;
    this.message = message;
  }

  static fromTravelmux(tError: TravelmuxError): TripFetchError {
    switch (tError.errorCode) {
      case TravelmuxErrorCode.ValhallaUnsupportedArea: {
        return {
          errorCode: TripFetchErrorCode.UnsupportedNonTransitArea,
          message: tError.message,
        };
      }
      case TravelmuxErrorCode.TransitUnsupportedArea: {
        return {
          errorCode: TripFetchErrorCode.UnsupportedTransitArea,
          message: tError.message,
        };
      }
      default: {
        return {
          errorCode: TripFetchErrorCode.Other,
          message: tError.message,
        };
      }
    }
  }
}

export async function fetchBestTrips(
  from: LngLat,
  to: LngLat,
  mode: TravelMode,
  distanceUnits: DistanceUnits,
  departureTime?: string,
  departureDate?: string,
  arriveBy?: boolean,
  transitWithBicycle?: boolean,
): Promise<Result<Trip[], TripFetchError>> {
  const modes = [mode];
  if (mode == TravelMode.Transit && transitWithBicycle) {
    modes.push(TravelMode.Bike);
  }
  const travelmuxModes = modes.map((m) => {
    switch (m) {
      case TravelMode.Walk:
        return TravelmuxMode.Walk;
      case TravelMode.Bike:
        return TravelmuxMode.Bike;
      case TravelMode.Drive:
        return TravelmuxMode.Drive;
      case TravelMode.Transit:
        return TravelmuxMode.Transit;
    }
  });

  return await TravelmuxClient.fetchPlans(
    from,
    to,
    travelmuxModes,
    5,
    distanceUnits,
    departureTime,
    departureDate,
    arriveBy,
  );
}

export const CircleStyles = {
  /// A small hollow dot, reading as a bead on the route's line.
  stop(color: string): CircleLayerSpecification['paint'] {
    return {
      'circle-radius': 3.5,
      'circle-color': '#ffffff',
      'circle-stroke-color': color,
      'circle-stroke-width': 2.5,
    };
  },
  /// The same bead, faded to match the line beyond the ridden portion.
  contextStop(color: string): CircleLayerSpecification['paint'] {
    return {
      ...CircleStyles.stop(color),
      'circle-opacity': 0.45,
      'circle-stroke-opacity': 0.45,
    };
  },
  /// The same bead, enlarged and heavily ringed, for a stop the rider gets on or off at.
  usedStop(color: string): CircleLayerSpecification['paint'] {
    return {
      'circle-radius': 5,
      'circle-color': '#ffffff',
      'circle-stroke-color': color,
      'circle-stroke-width': 5,
    };
  },
};

export const LineStyles = {
  /// Narrower and mostly transparent, so the ridden portion drawn over it reads as the emphasized
  /// part of the same line.
  context(color: string): LineLayerSpecification['paint'] {
    return {
      'line-color': color,
      'line-width': 5,
      'line-opacity': 0.45,
    };
  },
  activeColored(color: string): LineLayerSpecification['paint'] {
    return {
      'line-color': color,
      'line-width': 8,
    };
  },
  active: {
    'line-color': '#1296FF',
    'line-width': 8,
  },
  inactive: {
    'line-color': '#6FC1EE',
    'line-width': 4,
  },
  walkingActive: {
    'line-color': '#1296FF',
    'line-dasharray': [0, 1.5],
    'line-width': 6,
  },
  walkingInactive: {
    'line-color': '#6FC1EE',
    'line-dasharray': [0, 1.5],
    'line-width': 4,
  },
};
