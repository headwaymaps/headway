import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
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

  get firstTransitLeg(): TripLeg | undefined {
    return this.legs.slice(0, 2).find((leg) => leg.transitLeg);
  }
}

/// Alerts sharing a header, presented as one collapsible row.
export interface TransitAlertGroup {
  headerText?: string;
  alerts: TransitAlert[];
}

export class TripLeg {
  readonly raw: TravelmuxLeg;
  geometry: GeoJSON.LineString;

  constructor(raw: TravelmuxLeg) {
    this.raw = raw;
    const points = decodePolyline(this.raw.geometry, 6);
    this.geometry = {
      type: 'LineString',
      coordinates: points,
    };
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
    switch (this.raw.transitLeg?.vehicleMode) {
      case undefined:
        // not a transit leg - the traveler gets there themselves
        switch (this.raw.mode) {
          case TravelmuxMode.Bike:
            return '🚲';
          case TravelmuxMode.Drive:
            return '🚙';
          default:
            return '🚶‍♀️';
        }
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
    return new LngLat(this.raw.fromPlace.lon, this.raw.fromPlace.lat);
  }

  get destinationLngLat(): LngLat {
    return new LngLat(this.raw.toPlace.lon, this.raw.toPlace.lat);
  }

  get departureLocationName(): string | undefined {
    return this.raw.fromPlace.name;
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

export const LineStyles = {
  activeColored(color: string): LineLayerSpecification['paint'] {
    return {
      'line-color': color,
      'line-width': 6,
    };
  },
  active: {
    'line-color': '#1296FF',
    'line-width': 6,
  },
  inactive: {
    'line-color': '#6FC1EE',
    'line-width': 4,
  },
  walkingActive: {
    'line-color': '#1296FF',
    'line-dasharray': [0, 1.5],
    'line-width': 8,
  },
  walkingInactive: {
    'line-color': '#6FC1EE',
    'line-dasharray': [0, 1.5],
    'line-width': 8,
  },
};
