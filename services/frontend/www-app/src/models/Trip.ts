import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Result } from 'src/utils/Result';
import Itinerary, { ItineraryError } from './Itinerary';
import Route, { RouteError } from './Route';
import {
  TravelmuxMode,
  TravelmuxClient,
  TravelmuxItinerary,
  TravelmuxLeg,
  travelModeFromTravelmuxMode,
} from 'src/services/TravelmuxClient';
import { formatDistance, formatDuration } from 'src/utils/format';
import { decodePolyline } from 'src/third_party/decodePath';

export default class Trip {
  raw: TravelmuxItinerary;
  inner: Route | Itinerary;
  preferredDistanceUnits: DistanceUnits;
  innerDistanceUnits: DistanceUnits;
  legs: TripLeg[];

  constructor(
    raw: TravelmuxItinerary,
    preferredDistanceUnits: DistanceUnits,
    inner: Route | Itinerary,
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

  get distanceFormatted(): string | undefined {
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

  nonTransitRoute(): Route | undefined {
    if (this.mode != TravelMode.Transit) {
      return this.inner as Route;
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
    const points = decodePolyline(this.raw.geometry, 6, false);
    this.geometry = {
      type: 'LineString',
      coordinates: points,
    };
  }

  get start(): LngLat {
    const lngLat = this.geometry.coordinates[0];
    return new LngLat(lngLat[0], lngLat[1]);
  }

  get mode(): TravelMode {
    return travelModeFromTravelmuxMode(this.raw.mode);
  }

  paintStyle(active: boolean): LineLayerSpecification['paint'] {
    if (active) {
      if (this.mode == TravelMode.Walk || this.mode == TravelMode.Bike) {
        return LineStyles.walkingActive;
      } else {
        if (this.raw.routeColor) {
          return LineStyles.activeColored(`#${this.raw.routeColor}`);
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

export type TripFetchError =
  | { transit: true; itineraryError: ItineraryError }
  | { transit: false; routeError: RouteError };

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
    'line-color': '#1976D2',
    'line-width': 6,
  },
  inactive: {
    'line-color': '#777',
    'line-width': 4,
  },
  walkingActive: {
    'line-color': '#1976D2',
    'line-dasharray': [0, 1.5],
    'line-width': 8,
  },
  walkingInactive: {
    'line-color': '#777',
    'line-dasharray': [0, 1.5],
    'line-width': 8,
  },
};
