import { LngLat, LngLatLike } from 'maplibre-gl';
import { isEqual } from 'lodash';
import { DistanceUnits } from './models';
import Place from 'src/models/Place';

export default class Prefs {
  private static _stored: Prefs | undefined;
  static get stored(): Prefs {
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
  get mostRecentMapZoom(): number | null {
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
    } catch {
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
  get mostRecentMapCenter(): LngLatLike | null {
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
    } catch {
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

  setMostRecentMapCenter(lnglat: LngLatLike): void {
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

  private _mostRecentDistanceUnits: DistanceUnits | undefined | null;
  setMostRecentDistanceUnits(distanceUnits: DistanceUnits): void {
    if (!Object.values(DistanceUnits).includes(distanceUnits)) {
      throw new Error(`invalid DistanceUnits ${distanceUnits}`);
    }
    this.storage.setItem('distanceUnits', distanceUnits);
    this._mostRecentDistanceUnits = distanceUnits as DistanceUnits;
  }
  get mostRecentDistanceUnits(): DistanceUnits | null {
    if (this._mostRecentDistanceUnits !== undefined) {
      return this._mostRecentDistanceUnits;
    }
    const storedValue = this.storage.getItem('distanceUnits');
    if (!storedValue) {
      return null;
    }

    if (!Object.values(DistanceUnits).includes(storedValue as DistanceUnits)) {
      throw new Error(`invalid DistanceUnits ${storedValue}`);
    }

    const distanceUnits = storedValue as DistanceUnits;
    this._mostRecentDistanceUnits = distanceUnits;
    return distanceUnits;
  }

  private _mapScaleDistanceUnits: DistanceUnits | undefined | null;
  setMapScaleDistanceUnits(distanceUnits: DistanceUnits): void {
    if (!Object.values(DistanceUnits).includes(distanceUnits)) {
      throw new Error(`invalid mapScaleDistanceUnits ${distanceUnits}`);
    }
    this.storage.setItem('mapScaleDistanceUnits', distanceUnits);
    this._mapScaleDistanceUnits = distanceUnits as DistanceUnits;
  }

  get mapScaleDistanceUnits(): DistanceUnits | null {
    if (this._mostRecentDistanceUnits !== undefined) {
      return this._mostRecentDistanceUnits;
    }
    const storedValue = this.storage.getItem('mapScaleDistanceUnits');
    if (!storedValue) {
      return null;
    }

    if (!Object.values(DistanceUnits).includes(storedValue as DistanceUnits)) {
      throw new Error(`invalid mapScaleDistanceUnits ${storedValue}`);
    }

    const distanceUnits = storedValue as DistanceUnits;
    this._mapScaleDistanceUnits = distanceUnits;
    return distanceUnits;
  }

  distanceUnits(from: Place, to?: Place): DistanceUnits {
    const distanceUnits =
      from.preferredDistanceUnits() || to?.preferredDistanceUnits();
    if (distanceUnits) {
      if (this.mostRecentDistanceUnits != distanceUnits) {
        this.setMostRecentDistanceUnits(distanceUnits);
      }
      return distanceUnits;
    } else if (this.mostRecentDistanceUnits) {
      return this.mostRecentDistanceUnits;
    } else {
      console.warn('Assuming KM since no known or stored distance units');
      return DistanceUnits.Kilometers;
    }
  }
}
