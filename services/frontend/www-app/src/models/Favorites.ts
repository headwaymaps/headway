import { ref } from 'vue';
import { createUuid } from 'src/utils/uuid';
import { LngLat } from 'maplibre-gl';
import Place, { PlaceId, PlaceStorage } from './Place';

export interface Favorite {
  id: string;
  placeId: string;
  coordinates: [number, number];
  name: string;
  description: string;
  address?: string;
  phone?: string;
  website?: string;
  openingHours?: string;
  countryCode?: string;
  area?: string;
  createdAt: string;
}
const KEY = 'headway.favorites.v1';
export function readFavorites(storage: Storage): Favorite[] {
  try {
    const data: unknown = JSON.parse(storage.getItem(KEY) || '[]');
    if (!Array.isArray(data)) return [];
    return data.filter(
      (f): f is Favorite =>
        f &&
        typeof f.id === 'string' &&
        typeof f.placeId === 'string' &&
        typeof f.name === 'string' &&
        f.name.trim().length > 0 &&
        typeof f.description === 'string' &&
        typeof f.createdAt === 'string' &&
        [
          'address',
          'phone',
          'website',
          'openingHours',
          'countryCode',
          'area',
        ].every((key) => f[key] === undefined || typeof f[key] === 'string') &&
        Array.isArray(f.coordinates) &&
        f.coordinates.length === 2 &&
        f.coordinates.every(
          (n: unknown) => typeof n === 'number' && Number.isFinite(n),
        ) &&
        Math.abs(f.coordinates[0]) <= 180 &&
        Math.abs(f.coordinates[1]) <= 90,
    );
  } catch {
    return [];
  }
}
function load(): Favorite[] {
  try {
    return readFavorites(window.localStorage);
  } catch {
    return [];
  }
}
export const favorites = ref<Favorite[]>(load());
if (typeof window !== 'undefined')
  window.addEventListener('storage', (event) => {
    if (event.key === KEY || event.key === null) favorites.value = load();
  });
export function favoriteFor(place: Place): Favorite | undefined {
  const exact = favorites.value.find((f) => f.placeId === place.serializedId());
  if (exact) return exact;
  // A coordinate pin can refer to a saved business, but distinct business IDs
  // must never share a favorite simply because their coordinates coincide.
  if (place.id.gid) return undefined;
  const matches = favorites.value.filter(
    (f) =>
      f.coordinates[0] === place.point.lng &&
      f.coordinates[1] === place.point.lat,
  );
  return matches.length === 1 ? matches[0] : undefined;
}
function persist(next: Favorite[]) {
  // Commit in memory only after storage succeeds, so a failed save is visible.
  window.localStorage.setItem(KEY, JSON.stringify(next));
  favorites.value = next;
}
function unusedFavoriteId(): string {
  const existingIds = new Set(favorites.value.map((favorite) => favorite.id));
  for (let attempt = 0; attempt < 10; attempt++) {
    const id = createUuid();
    if (!existingIds.has(id)) return id;
  }
  throw new Error('Could not generate a unique favorite ID. Please try again.');
}
export function saveFavorite(
  place: Place,
  name: string,
  description: string,
): Favorite {
  if (!name.trim()) throw new Error('Enter a name for this favorite.');
  const previous = favoriteFor(place);
  const favorite: Favorite = {
    id: previous?.id ?? unusedFavoriteId(),
    placeId: previous?.placeId ?? place.serializedId(),
    coordinates: [place.point.lng, place.point.lat],
    name: name.trim(),
    description: description.trim(),
    address: place.address ?? undefined,
    phone: place.phone,
    website: place.website,
    openingHours: place.openingHours,
    countryCode: place.countryCode,
    area: place.area,
    createdAt: previous?.createdAt ?? new Date().toISOString(),
  };
  persist([...favorites.value.filter((f) => f.id !== favorite.id), favorite]);
  return favorite;
}
export function removeFavorite(id: string) {
  persist(favorites.value.filter((f) => f.id !== id));
}
export function favoritePlace(favorite: Favorite): Place {
  return new Place(
    PlaceId.deserialize(favorite.placeId),
    new LngLat(...favorite.coordinates),
    undefined,
    {
      name: favorite.name,
      address: favorite.address,
      phone: favorite.phone,
      website: favorite.website,
      openingHours: favorite.openingHours,
      countryCode: favorite.countryCode,
      area: favorite.area,
    },
  );
}

/** Fetch current indexed business information, bypassing the in-memory cache. */
export async function refreshFavoritePlace(favorite: Favorite): Promise<Place> {
  const latest = await PlaceStorage.fetchFromId(
    PlaceId.deserialize(favorite.placeId),
    true,
  );
  return new Place(latest.id, latest.point, latest.bbox, {
    name: favorite.name,
    address: latest.address ?? undefined,
    phone: latest.phone,
    website: latest.website,
    openingHours: latest.openingHours,
    countryCode: latest.countryCode,
    area: latest.area,
  });
}
