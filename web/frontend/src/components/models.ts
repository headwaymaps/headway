export interface POI {
  key?: string;
  name?: string | null;
  address?: string | null;
  position?: LongLat;
  id?: number;
  type?: string | null;
}

export interface LongLat {
  long: number;
  lat: number;
}

// FIXME: this is US-only, if we get international users it must be expanded.
export function localizeAddress(
  houseNumber: string | undefined | null,
  road: string | undefined | null,
  neighborhood: string | undefined | null,
  city: string | undefined | null
) {
  if (houseNumber && road && neighborhood && city) {
    return `${houseNumber} ${road}, ${neighborhood}, ${city}`;
  }
  if (houseNumber && road && city) {
    return `${houseNumber} ${road}, ${city}`;
  }
  if (houseNumber && road) {
    return `${houseNumber} ${road}`;
  }
  if (road) {
    return `${houseNumber} ${road}`;
  }
  if (neighborhood && city) {
    return `${neighborhood}, ${city}`;
  }
  if (city) {
    return `${city}`;
  }
  return undefined;
}
