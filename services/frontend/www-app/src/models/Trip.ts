import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Err, Ok, Result } from 'src/utils/Result';
import Itinerary, { ItineraryError } from './Itinerary';
import Route, { RouteError } from './Route';

export default interface Trip {
  durationFormatted: string;
  lengthFormatted?: string;
  bounds: LngLatBounds;
  legs: TripLeg[];
  mode: TravelMode;
}

export interface TripLeg {
  geometry(): GeoJSON.LineString;
  paintStyle(active: boolean): LineLayerSpecification['paint'];
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
  departureDate?: string
): Promise<Result<Trip[], TripFetchError>> {
  switch (mode) {
    case TravelMode.Walk:
    case TravelMode.Bike:
    case TravelMode.Drive: {
      const result = await Route.getRoutes(from, to, mode, distanceUnits);
      if (result.ok) {
        return Ok(result.value);
      } else {
        const routeError = result.error;
        return Err({ transit: false, routeError });
      }
    }
    case TravelMode.Transit: {
      const result = await Itinerary.fetchBest(
        from,
        to,
        distanceUnits,
        departureTime,
        departureDate
      );
      if (result.ok) {
        return Ok(result.value);
      } else {
        const itineraryError = result.error;
        return Err({ transit: true, itineraryError });
      }
    }
  }
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
