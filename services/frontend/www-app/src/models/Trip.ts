import { LineLayerSpecification, LngLat } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Result } from 'src/utils/Result';
import { ItineraryError } from './Itinerary';
import { RouteError } from './Route';
import {
  TravelmuxMode,
  TravelmuxClient,
  TravelmuxTrip,
} from 'src/services/TravelmuxClient';

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
): Promise<Result<TravelmuxTrip[], TripFetchError>> {
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
