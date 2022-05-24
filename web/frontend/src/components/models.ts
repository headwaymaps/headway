export interface POI {
  key: string;
  name: string;
  caption?: string;
  position?: LongLat;
}

export interface LongLat {
  long: number;
  lat: number;
}
