import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Result } from 'src/utils/Result';
import Itinerary from './Itinerary';
import {
  TravelmuxMode,
  TravelmuxClient,
  TravelmuxItinerary,
  TravelmuxLeg,
  travelModeFromTravelmuxMode,
  TravelmuxError,
  TravelmuxErrorCode,
} from 'src/services/TravelmuxClient';
import { formatDistance, formatDuration } from 'src/utils/format';
import { decodePolyline } from 'src/utils/decodePolyline';
import { i18n } from 'src/i18n/lang';

export default class Trip {
  raw: TravelmuxItinerary;
  inner: Itinerary | null;
  preferredDistanceUnits: DistanceUnits;
  innerDistanceUnits: DistanceUnits;
  legs: TripLeg[];

  constructor(
    raw: TravelmuxItinerary,
    preferredDistanceUnits: DistanceUnits,
    inner: Itinerary | null,
    innerDistanceUnits: DistanceUnits,
  ) {
    this.raw = raw;
    this.preferredDistanceUnits = preferredDistanceUnits;
    this.inner = inner;
    this.legs = raw.legs.map((raw: TravelmuxLeg) => new TripLeg(raw));
    this.innerDistanceUnits = innerDistanceUnits;
  }

  get durationFormatted(): string {
    return formatDuration(this.raw.duration, 'shortform');
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
      this.raw.distance,
      this.innerDistanceUnits,
      this.preferredDistanceUnits,
    );
  }

  get bounds(): LngLatBounds {
    return new LngLatBounds(this.raw.bounds.min, this.raw.bounds.max);
  }

  get mode(): TravelMode {
    return travelModeFromTravelmuxMode(this.raw.mode);
  }

  transitItinerary(): Itinerary | undefined {
    if (this.mode == TravelMode.Transit) {
      return this.inner as Itinerary;
    } else {
      return undefined;
    }
  }
}

export class TripLeg {
  raw: TravelmuxLeg;
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

  paintStyle(active: boolean): LineLayerSpecification['paint'] {
    if (active) {
      if (this.mode == TravelMode.Walk || this.mode == TravelMode.Bike) {
        return LineStyles.walkingActive;
      } else {
        if (this.raw.transitLeg?.routeColor) {
          return LineStyles.activeColored(
            `#${this.raw.transitLeg?.routeColor}`,
          );
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
