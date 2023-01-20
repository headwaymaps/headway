import { expect, test } from '@jest/globals';
import { LngLat } from 'maplibre-gl';
import Place from 'src/models/Place';
import { DistanceUnits } from './models';
import Prefs from './Prefs';

/**
 * For testing without using localStorage.
 */
class UnpersistedStorage implements Storage {
  items = new Map<string, string>();

  get length(): number {
    return this.items.size;
  }

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  key(index: number): string {
    throw 'unimplemented';
  }

  getItem(key: string): string | null {
    const value = this.items.get(key);
    if (value === undefined) {
      // conform to Storage interface
      return null;
    } else {
      return value;
    }
  }

  setItem(key: string, value: string) {
    this.items.set(key, value);
  }

  removeItem(key: string) {
    this.items.delete(key);
  }

  clear() {
    this.items.clear();
  }
}

test('unpersisted storage', () => {
  const storage = new UnpersistedStorage();
  expect(storage.getItem('foo')).toBeNull();
  expect(storage.length).toBe(0);

  storage.setItem('foo', 'bar');
  expect(storage.getItem('foo')).toBe('bar');
  expect(storage.length).toBe(1);

  storage.removeItem('foo');
  expect(storage.getItem('foo')).toBeNull();
  expect(storage.length).toBe(0);

  storage.setItem('foo', 'bar');
  storage.clear();
  expect(storage.getItem('foo')).toBeNull();
  expect(storage.length).toBe(0);
});

test('most recent center', () => {
  const storage = new UnpersistedStorage();
  const prefs = new Prefs(storage);
  expect(prefs.mostRecentMapCenter).toBe(null);

  prefs.setMostRecentMapCenter(new LngLat(1.0, 2.0));
  expect(prefs.mostRecentMapCenter).toStrictEqual([1.0, 2.0]);

  prefs.setMostRecentMapCenter(new LngLat(3.0, 4.0));
  prefs.setMostRecentMapCenter([5.0, 6.0]);
  expect(prefs.mostRecentMapCenter).toStrictEqual([5.0, 6.0]);
});

test('distance units default', () => {
  const storage = new UnpersistedStorage();
  const fromPlace = Place.bareLocation(new LngLat(0, 0));
  const prefs = new Prefs(storage);
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Kilometers);
});

test('distance units from fromPlace (mi)', () => {
  const storage = new UnpersistedStorage();
  const fromPlace = Place.bareLocation(new LngLat(0, 0));
  fromPlace.countryCode = 'US';
  const prefs = new Prefs(storage);
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Miles);
});

test('distance units from fromPlace (km)', () => {
  const storage = new UnpersistedStorage();
  const fromPlace = Place.bareLocation(new LngLat(0, 0));
  fromPlace.countryCode = 'CA';
  const prefs = new Prefs(storage);
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Kilometers);
});

test('distance units prefer fromPlace over toPlace', () => {
  const storage = new UnpersistedStorage();
  const fromPlace = Place.bareLocation(new LngLat(0, 0));
  fromPlace.countryCode = 'US';
  const toPlace = Place.bareLocation(new LngLat(0, 0));
  toPlace.countryCode = 'CA';
  const prefs = new Prefs(storage);
  expect(prefs.distanceUnits(fromPlace, toPlace)).toBe(DistanceUnits.Miles);
});

test('distance units from toPlace', () => {
  const storage = new UnpersistedStorage();
  const fromPlace = Place.bareLocation(new LngLat(0, 0));
  fromPlace.countryCode = undefined;
  const toPlace = Place.bareLocation(new LngLat(0, 0));
  fromPlace.countryCode = 'US';
  const prefs = new Prefs(storage);
  expect(prefs.distanceUnits(fromPlace, toPlace)).toBe(DistanceUnits.Miles);
});

test('distance units from storage', () => {
  const storage = new UnpersistedStorage();
  const prefs = new Prefs(storage);

  // sanity check that we default to KM
  const fromPlace = Place.bareLocation(new LngLat(0, 0));
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Kilometers);

  // specify country code to get different units
  fromPlace.countryCode = 'US';
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Miles);

  // remember past units
  fromPlace.countryCode = undefined;
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Miles);

  // specifying new country code forces new units
  fromPlace.countryCode = 'CA';
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Kilometers);

  // And that becomes the new default
  fromPlace.countryCode = undefined;
  expect(prefs.distanceUnits(fromPlace)).toBe(DistanceUnits.Kilometers);
});
