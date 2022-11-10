import { LngLat } from 'maplibre-gl';

export interface LongLat {
  long: number;
  lat: number;
}

/// Unfortunately we use (at least) two different objects for storing latitude and longitude.
/// So this is a helper to convert between two of them.
/// Long term I'd like to consolidate around fewer.
export function toLngLat(longLat: LongLat): LngLat {
  return new LngLat(longLat.long, longLat.lat);
}

export function fastDistanceMeters(p1: LongLat, p2: LongLat): number {
  const averageLatRadians = (Math.PI * (p1.lat + p2.lat)) / 360;
  const latMult = Math.cos(averageLatRadians);
  const earthRadiusMeters = 6371000;
  return (
    earthRadiusMeters *
    Math.sqrt(
      Math.pow((latMult * Math.PI * (p1.long - p2.long)) / 180, 2) +
        Math.pow((Math.PI * (p1.lat - p2.lat)) / 180, 2)
    )
  );
}

export function fastPolylineDistanceMeters(polyline: LongLat[]) {
  let dist = 0;
  for (let i = 1; i < polyline.length; i += 1) {
    dist += fastDistanceMeters(polyline[i - 1], polyline[i]);
  }
  return dist;
}
