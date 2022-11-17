import { DistanceUnits } from '../utils/models';
import PeliasClient from 'src/services/PeliasClient';
import { LngLat } from 'maplibre-gl';

/// PlaceId can be either a LngLat or a gid (but not both).
type GID = string;
class PlaceId {
  public readonly location?: LngLat;
  public readonly gid?: GID;

  constructor(location?: LngLat, gid?: GID) {
    if (location && gid) {
      throw new Error('PlaceId cannot have both location and gid');
    }
    if (!location && !gid) {
      throw new Error('PlaceId requires either location or gid');
    }

    this.location = location;
    this.gid = gid;
  }

  static location(location: LngLat): PlaceId {
    return new PlaceId(location, undefined);
  }

  static gid(gid: GID): PlaceId {
    return new PlaceId(undefined, gid);
  }

  serialized(): string {
    if (this.location) {
      return `${this.location.lng},${this.location.lat}`;
    } else {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      return this.gid!;
    }
  }

  public static deserialize(serialized: string): PlaceId {
    if (/([0-9\.-]+,[0-9\.-]+)/.test(serialized)) {
      const longLat = serialized.split(',');
      const location = new LngLat(
        parseFloat(longLat[0]),
        parseFloat(longLat[1])
      );
      return PlaceId.location(location);
    } else {
      return PlaceId.gid(serialized);
    }
  }

  get type(): string {
    if (location) {
      return 'location';
    } else {
      return 'gid';
    }
  }
}

/// Wrapper around a pelias response
export default class Place {
  id: PlaceId;
  point: LngLat;
  countryCode?: string;

  constructor(id: PlaceId, point: LngLat, countryCode?: string) {
    this.id = id;
    this.point = point;
    this.countryCode = countryCode;
  }

  public static async fetchFromSerializedId(
    serializedId: string
  ): Promise<Place> {
    const id = PlaceId.deserialize(serializedId);
    return Place.fetchFromId(id);
  }

  public static async fetchFromId(id: PlaceId): Promise<Place> {
    const placeJson = await (async () => {
      if (id.gid) {
        return await PeliasClient.findByGid(id.gid);
      } else if (id.location) {
        return await PeliasClient.findByLocation(id.location);
      } else {
        throw new Error(`Invalid PlaceId: ${id}`);
      }
    })();

    const feature = placeJson.features[0];
    console.assert(feature, 'no feature found for placeId', id);

    const geometry = feature.geometry as GeoJSON.Point;
    console.assert(geometry, 'no geometry found for feature', feature);
    console.assert(
      geometry.type == 'Point',
      'unexpected geometry found for feature',
      geometry
    );
    const [lng, lat] = geometry.coordinates;
    console.assert(lng, 'missing lng');
    console.assert(lat, 'missing lat');
    const location = new LngLat(lng, lat);

    const countryCode = feature.properties?.country_code;
    console.assert(countryCode, 'no country code found for feature', feature);

    return new Place(id, location, countryCode);
  }

  public preferredDistanceUnits(): DistanceUnits | undefined {
    const imperialDogs = ['US', 'GB', 'LR', 'MM'];
    if (!this.countryCode) {
      return undefined;
    }

    if (imperialDogs.includes(this.countryCode)) {
      return DistanceUnits.Miles;
    } else {
      return undefined;
    }
  }
}
