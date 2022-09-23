import { LngLat, LngLatLike } from 'maplibre-gl';
import { isEqual } from 'lodash';

export default class Prefs {
  private static _stored: Prefs | undefined;
  static stored(): Prefs {
    if (Prefs._stored === undefined) {
      Prefs._stored = new Prefs(window.localStorage);
    }
    return Prefs._stored;
  }

  private storage: Storage;

  constructor(storage: Storage) {
    this.storage = storage;
  }

  private _mostRecentMapZoom: number | undefined | null;
  mostRecentMapZoom(): number | null {
    if (this._mostRecentMapZoom !== undefined) {
      return this._mostRecentMapZoom;
    }
    const json = this.storage.getItem('mostRecentMapZoom');
    if (!json) {
      return null;
    }
    let value;
    try {
      value = JSON.parse(json);
    } catch (e) {
      console.warn('invalid json stored for map zoom:', json);
      return null;
    }

    if (typeof value === 'number') {
      this._mostRecentMapZoom = value;
      return value;
    } else {
      console.warn('invalid value stored for map zoom:', value);
      return null;
    }
  }

  setMostRecentMapZoom(zoom: number) {
    if (zoom == this._mostRecentMapZoom) {
      // no-op, avoid writing to storage.
      return;
    }

    const json = JSON.stringify(zoom);
    this.storage.setItem('mostRecentMapZoom', json);
    this._mostRecentMapZoom = zoom;
  }

  private _mostRecentMapCenter: LngLatLike | undefined | null;
  mostRecentMapCenter(): LngLatLike | null {
    if (this._mostRecentMapCenter !== undefined) {
      return this._mostRecentMapCenter;
    }

    const json = this.storage.getItem('mostRecentMapCenter');
    if (!json) {
      return null;
    }

    let value;
    try {
      value = JSON.parse(json);
    } catch (e) {
      console.warn('invalid json stored for map center:', json);
      return null;
    }

    if (Array.isArray(value) && value.length == 2) {
      const center = value as [number, number];
      this._mostRecentMapCenter = center;
      return center;
    } else {
      console.warn('invalid value stored for map center:', value);
      return null;
    }
  }

  setMostRecentMapCenter(lnglat: LngLatLike) {
    const coords = LngLat.convert(lnglat).toArray();

    if (isEqual(coords, [0, 0])) {
      // don't store null island
      return;
    }

    if (isEqual(coords, this._mostRecentMapCenter)) {
      // no-op, avoid writing to storage.
      return;
    }

    const json = JSON.stringify(coords);
    this.storage.setItem('mostRecentMapCenter', json);
    this._mostRecentMapCenter = coords as [number, number];
  }
}
