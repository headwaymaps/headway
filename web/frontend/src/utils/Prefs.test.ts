import { expect, test } from '@jest/globals';
import { LngLat } from 'maplibre-gl';
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
  expect(prefs.mostRecentMapCenter()).toBe(null);

  prefs.setMostRecentMapCenter(new LngLat(1.0, 2.0));
  expect(prefs.mostRecentMapCenter()).toStrictEqual([1.0, 2.0]);

  prefs.setMostRecentMapCenter(new LngLat(3.0, 4.0));
  prefs.setMostRecentMapCenter([5.0, 6.0]);
  expect(prefs.mostRecentMapCenter()).toStrictEqual([5.0, 6.0]);
});
