import { afterEach, expect, test, vi } from 'vitest';
import { LngLat } from 'maplibre-gl';
import Place, { PlaceId, PlaceStorage } from './Place';
import {
  favorites,
  favoriteFor,
  favoritePlace,
  readFavorites,
  removeFavorite,
  saveFavorite,
  refreshFavoritePlace,
} from './Favorites';

const items = new Map<string, string>();
const storage = {
  getItem: (key: string) => items.get(key) ?? null,
  setItem: (key: string, value: string) => {
    items.set(key, value);
  },
} as Storage;
function setup() {
  vi.stubGlobal('window', { localStorage: storage });
  return new Place(PlaceId.gid('osm:test'), new LngLat(-122, 47), undefined, {
    name: 'Cafe',
    address: 'Main Street',
  });
}
afterEach(() => {
  favorites.value = [];
  items.clear();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

test('favorites persist custom text and restore without geocoding', () => {
  const place = setup();
  const saved = saveFavorite(place, ' My cafe ', ' Quiet upstairs ');
  const loaded = readFavorites(storage);
  expect(loaded).toEqual([saved]);
  expect(saved.name).toBe('My cafe');
  expect(saved.description).toBe('Quiet upstairs');
  expect(favoritePlace(loaded[0]!).point.toArray()).toEqual([-122, 47]);
  expect(favoritePlace(loaded[0]!).address).toBe('Main Street');
});
test('saving the same place edits it and removing it persists', () => {
  const place = setup();
  const first = saveFavorite(place, 'Cafe', '');
  const edited = saveFavorite(place, 'Work spot', 'Wifi');
  expect(edited.id).toBe(first.id);
  expect(favorites.value).toHaveLength(1);
  expect(favoriteFor(place)?.name).toBe('Work spot');
  removeFavorite(first.id);
  expect(readFavorites(storage)).toEqual([]);
});
test('corrupt storage and invalid coordinates are ignored', () => {
  setup();
  storage.setItem('headway.favorites.v1', '{bad');
  expect(readFavorites(storage)).toEqual([]);
  const saved = saveFavorite(
    new Place(PlaceId.gid('a'), new LngLat(0, 0)),
    'Zero',
    '',
  );
  storage.setItem(
    'headway.favorites.v1',
    JSON.stringify([saved, { ...saved, coordinates: [0, 91] }]),
  );
  expect(readFavorites(storage)).toEqual([saved]);
});
test('failed persistence does not report an in-memory save', () => {
  const place = setup();
  vi.stubGlobal('window', {
    localStorage: {
      setItem() {
        throw new Error('quota');
      },
    },
  });
  expect(() => saveFavorite(place, 'Cafe', '')).toThrow('quota');
  expect(favorites.value).toEqual([]);
});

test('blank names leave existing data unchanged', () => {
  const place = setup();
  const saved = saveFavorite(place, 'Cafe', '');
  expect(() => saveFavorite(place, ' \n ', 'notes')).toThrow('Enter a name');
  expect(readFavorites(storage)).toEqual([saved]);
});

test('editing preserves identity, creation time and unrelated favorites', () => {
  const place = setup();
  const first = saveFavorite(place, 'Cafe', '');
  const other = saveFavorite(
    new Place(PlaceId.gid('other'), new LngLat(1, 2)),
    'Other',
    '',
  );
  const edited = saveFavorite(place, 'New name', 'New notes');
  expect(edited.id).toBe(first.id);
  expect(edited.createdAt).toBe(first.createdAt);
  expect(readFavorites(storage)).toEqual([other, edited]);
  removeFavorite(edited.id);
  expect(readFavorites(storage)).toEqual([other]);
});

test('a dropped pin at an already saved place updates the same favorite', () => {
  const place = setup();
  const first = saveFavorite(place, 'Cafe', '');
  const pin = new Place(PlaceId.location(place.point), place.point);
  expect(favoriteFor(pin)?.id).toBe(first.id);
  expect(saveFavorite(pin, 'Pinned cafe', '').id).toBe(first.id);
  expect(favorites.value).toHaveLength(1);
});

test('metadata survives restoration and editing', () => {
  const place = setup();
  Object.assign(place, {
    phone: '123',
    website: 'https://example.com',
    openingHours: '24/7',
    countryCode: 'US',
    area: 'Seattle',
  });
  const restored = favoritePlace(saveFavorite(place, 'My cafe', 'Notes'));
  for (const key of [
    'phone',
    'website',
    'openingHours',
    'countryCode',
    'area',
    'address',
  ] as const)
    expect(restored[key]).toBe(place[key]);
  expect(saveFavorite(restored, 'Renamed', 'Edited').website).toBe(
    place.website,
  );
});

test.each([null, {}, 'text', 5])('non-array storage %j is ignored', (value) => {
  setup();
  storage.setItem('headway.favorites.v1', JSON.stringify(value));
  expect(readFavorites(storage)).toEqual([]);
});

test('malformed entries are ignored without losing valid entries', () => {
  const saved = saveFavorite(setup(), 'Cafe', '');
  const invalid = [
    null,
    {},
    { ...saved, name: ' ' },
    { ...saved, description: 1 },
    { ...saved, coordinates: [181, 0] },
    { ...saved, coordinates: ['0', 0] },
    { ...saved, coordinates: [0] },
    { ...saved, phone: 123 },
  ];
  storage.setItem('headway.favorites.v1', JSON.stringify([...invalid, saved]));
  expect(readFavorites(storage)).toEqual([saved]);
});

test('coordinate boundaries and Unicode notes round-trip', () => {
  setup();
  const point = new LngLat(180, 90);
  const saved = saveFavorite(
    new Place(PlaceId.location(point), point),
    '星 🌟',
    'First line\nSecond line <script>',
  );
  expect(readFavorites(storage)).toEqual([saved]);
  expect(favoritePlace(saved).point.toArray()).toEqual([180, 90]);
});

test('failed edits and removals leave existing state intact', () => {
  const place = setup();
  const saved = saveFavorite(place, 'Cafe', '');
  vi.stubGlobal('window', {
    localStorage: {
      setItem() {
        throw new Error('blocked');
      },
    },
  });
  expect(() => saveFavorite(place, 'Changed', '')).toThrow('blocked');
  expect(() => removeFavorite(saved.id)).toThrow('blocked');
  expect(favorites.value).toEqual([saved]);
  expect(readFavorites(storage)).toEqual([saved]);
});

test('storage read failures return an empty list', () => {
  expect(
    readFavorites({
      getItem() {
        throw new Error('blocked');
      },
    } as unknown as Storage),
  ).toEqual([]);
});

test('different businesses at identical coordinates keep separate personal fields', () => {
  const first = setup();
  const second = new Place(PlaceId.gid('osm:other-business'), first.point);
  const a = saveFavorite(first, 'My cafe', 'Coffee');
  expect(favoriteFor(second)).toBeUndefined();
  const b = saveFavorite(second, 'My bakery', 'Bread');
  expect(a.id).not.toBe(b.id);
  expect(favoriteFor(first)?.description).toBe('Coffee');
  expect(favoriteFor(second)?.description).toBe('Bread');
  const pin = new Place(PlaceId.location(first.point), first.point);
  expect(favoriteFor(pin)).toBeUndefined();
});

test('refresh bypasses cached data and keeps personal text while replacing business details', async () => {
  const place = setup();
  place.phone = 'old phone';
  const saved = saveFavorite(place, 'My own name', 'Personal notes');
  const fresh = new Place(place.id, new LngLat(-121, 48), undefined, {
    name: 'Official name',
    phone: 'new phone',
    address: 'New address',
    website: 'https://new.example',
  });
  const fetch = vi.spyOn(PlaceStorage, 'fetchFromId').mockResolvedValue(fresh);
  const result = await refreshFavoritePlace(saved);
  expect(fetch).toHaveBeenCalledWith(
    expect.objectContaining({ gid: place.id.gid }),
    true,
  );
  expect(result.name).toBe('My own name');
  expect(result.phone).toBe('new phone');
  expect(result.address).toBe('New address');
  expect(result.website).toBe('https://new.example');
  expect(result.openingHours).toBeUndefined();
  expect(result.point.toArray()).toEqual([-121, 48]);
  expect(favoriteFor(result)?.description).toBe('Personal notes');
  expect(readFavorites(storage)).toEqual([saved]);
});
test('refresh failures propagate without modifying personal data', async () => {
  const saved = saveFavorite(setup(), 'Personal name', 'Notes');
  vi.spyOn(PlaceStorage, 'fetchFromId').mockRejectedValue(new Error('offline'));
  await expect(refreshFavoritePlace(saved)).rejects.toThrow('offline');
  expect(readFavorites(storage)).toEqual([saved]);
});

test('favorites can be created without randomUUID and edits do not request a new ID', () => {
  const place = setup();
  const getRandomValues = vi.fn((bytes: Uint8Array) => bytes.fill(42));
  vi.stubGlobal('crypto', { getRandomValues });
  const saved = saveFavorite(place, 'HTTP favorite', 'Notes');
  expect(saved.id).toMatch(
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
  );
  expect(readFavorites(storage)[0]?.id).toBe(saved.id);
  const edited = saveFavorite(place, 'Edited HTTP favorite', 'Updated notes');
  expect(edited.id).toBe(saved.id);
  expect(getRandomValues).toHaveBeenCalledOnce();
});

test('ID collisions retry against all saved IDs without overwriting favorites', () => {
  const firstPlace = setup();
  const randomUUID = vi
    .fn()
    .mockReturnValueOnce('first-id')
    .mockReturnValueOnce('second-id')
    .mockReturnValueOnce('first-id')
    .mockReturnValueOnce('second-id')
    .mockReturnValueOnce('third-id');
  vi.stubGlobal('crypto', { randomUUID });
  const first = saveFavorite(firstPlace, 'First', 'Keep first');
  const second = saveFavorite(
    new Place(PlaceId.gid('second'), new LngLat(1, 2)),
    'Second',
    'Keep second',
  );
  const third = saveFavorite(
    new Place(PlaceId.gid('third'), new LngLat(3, 4)),
    'Third',
    'New',
  );
  expect(third.id).toBe('third-id');
  expect(randomUUID).toHaveBeenCalledTimes(5);
  expect(readFavorites(storage)).toEqual([first, second, third]);
  expect(favorites.value).toEqual([first, second, third]);
});

test('repeated collisions stop after ten attempts without changing data', () => {
  const firstPlace = setup();
  const randomUUID = vi.fn(() => 'same-id');
  vi.stubGlobal('crypto', { randomUUID });
  const first = saveFavorite(firstPlace, 'First', 'Keep me');
  randomUUID.mockClear();
  expect(() =>
    saveFavorite(
      new Place(PlaceId.gid('other'), new LngLat(1, 2)),
      'Other',
      '',
    ),
  ).toThrow('unique favorite ID');
  expect(randomUUID).toHaveBeenCalledTimes(10);
  expect(favorites.value).toEqual([first]);
  expect(readFavorites(storage)).toEqual([first]);
});

test('editing a favorite reuses its ID without invoking the generator', () => {
  const place = setup();
  const first = saveFavorite(place, 'First', '');
  const randomUUID = vi.fn(() => {
    throw new Error('must not generate');
  });
  vi.stubGlobal('crypto', { randomUUID });
  const edited = saveFavorite(place, 'Edited', 'Updated');
  expect(edited.id).toBe(first.id);
  expect(randomUUID).not.toHaveBeenCalled();
  expect(readFavorites(storage)).toEqual([edited]);
});
