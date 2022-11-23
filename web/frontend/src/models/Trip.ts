import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import Itinerary from './Itinerary';
import Route from './Route';

export default interface Trip {
  durationFormatted: string;
  lengthFormatted?: string;
  bounds: LngLatBounds;
  legs: TripLeg[];
}

export interface TripLeg {
  geometry(): GeoJSON.LineString;
  paintStyle(active: boolean): LineLayerSpecification['paint'];
}

export async function fetchBestTrips(
  from: LngLat,
  to: LngLat,
  mode: TravelMode,
  distanceUnits: DistanceUnits
): Promise<Trip[]> {
  switch (mode) {
    case TravelMode.Walk:
    case TravelMode.Bike:
    case TravelMode.Drive: {
      return Route.getRoutes(from, to, mode, distanceUnits);
    }
    case TravelMode.Transit: {
      return Itinerary.fetchBest(from, to, distanceUnits);
    }
  }
}
