import { LngLat } from 'maplibre-gl';

type PlaceResponse = GeoJSON.FeatureCollection;

export default class PeliasClient {
  static async findByGid(gid: string): Promise<PlaceResponse> {
    const response = await fetch(`/pelias/v1/place?ids=${gid}`);
    if (response.ok) {
      return await response.json();
    } else {
      throw new Error(`error response from pelias: ${response}`);
    }
  }

  static async findByLocation(location: LngLat): Promise<PlaceResponse> {
    const response = await fetch(
      `/pelias/v1/reverse?point.lat=${location.lat}&point.lon=${location.lng}&boundary.circle.radius=0.1&sources=osm`
    );
    if (response.ok) {
      return await response.json();
    } else {
      throw new Error(`error response from pelias: ${response}`);
    }
  }
}
