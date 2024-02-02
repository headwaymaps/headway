import { LngLat } from 'maplibre-gl';

type PlaceResponse = GeoJSON.FeatureCollection;
type AutocompleteResponse = GeoJSON.FeatureCollection;

export default class PeliasClient {
  static async autocomplete(
    text: string,
    focus?: LngLat,
  ): Promise<AutocompleteResponse> {
    let url = `/pelias/v1/autocomplete?text=${encodeURIComponent(text)}`;
    if (focus) {
      url += `&focus.point.lon=${focus.lng}&focus.point.lat=${focus.lat}`;
    }
    const response = await fetch(url);

    if (response.ok) {
      return await response.json();
    } else {
      throw new Error(`error response from pelias: ${response}`);
    }
  }

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
      `/pelias/v1/reverse?point.lat=${location.lat}&point.lon=${location.lng}&boundary.circle.radius=0.1&sources=osm`,
    );
    if (response.ok) {
      return await response.json();
    } else {
      throw new Error(`error response from pelias: ${response}`);
    }
  }

  // This endpoint is unused for now. The results are very different and (in my
  // estimation) worse in some cases, so we only use the "autocomplete" search
  // for now.
  //
  // See https://github.com/pelias/pelias/issues/938
  static async search(
    text: string,
    focus?: LngLat,
  ): Promise<AutocompleteResponse> {
    let url = `/pelias/v1/search?text=${encodeURIComponent(text)}`;
    if (focus) {
      url += `&focus.point.lon=${focus.lng}&focus.point.lat=${focus.lat}`;
    }

    const response = await fetch(url);

    if (response.ok) {
      return await response.json();
    } else {
      throw new Error(`error response from pelias: ${response}`);
    }
  }
}
