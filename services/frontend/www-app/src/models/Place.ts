import { localizeAddress, DistanceUnits } from '../utils/models';
import PeliasClient from 'src/services/PeliasClient';
import { LngLat, LngLatBounds } from 'maplibre-gl';
import OSMID from './OSMID';

/// PlaceId can be either a LngLat or a gid (but not both).
type GID = string;
export class PlaceId {
  public readonly location?: LngLat | undefined;
  public readonly gid?: GID | undefined;

  constructor(location?: LngLat, gid?: GID) {
    if (location) {
      this.location = location;
    } else if (gid) {
      this.gid = gid;
    } else {
      throw new Error('PlaceId requires either location or gid');
    }
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
      return this.gid!;
    }
  }

  public urlEncoded(): string {
    return encodeURIComponent(this.serialized());
  }

  public static urlDecoded(urlEncoded: string): PlaceId {
    const decoded = decodeURIComponent(urlEncoded);
    return PlaceId.deserialize(decoded);
  }

  public static deserialize(serialized: string): PlaceId {
    if (/([0-9.-]+,[0-9.-]+)/.test(serialized)) {
      const lngLat = serialized.split(',');
      console.assert!(lngLat.length == 2, 'unexpected length', lngLat);
      const location = new LngLat(
        parseFloat(lngLat[0]!),
        parseFloat(lngLat[1]!),
      );
      return PlaceId.location(location);
    } else {
      return PlaceId.gid(serialized);
    }
  }

  get type(): string {
    if (this.location) {
      return 'location';
    } else {
      return 'gid';
    }
  }

  osmVenueId(): OSMID | undefined {
    if (!this.gid) {
      return undefined;
    }
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const [_, osmIdString] = this.gid.split('openstreetmap:venue:');
    if (osmIdString) {
      return OSMID.deserialize(osmIdString);
    }
  }

  public editOSMVenueUrl(): URL | undefined {
    // https://www.openstreetmap.org/edit?editor=id&node=12345
    const osmVenueId = this.osmVenueId();
    if (!osmVenueId) {
      return;
    }
    const url = new URL('https://www.openstreetmap.org/edit');
    url.searchParams.set('editor', 'id');
    url.searchParams.set(osmVenueId.idType, osmVenueId.idNumber.toString());
    return url;
  }
}

export class PlaceStorage {
  /// Cache of serialized PlaceId -> Place
  /// NOTE: I tried using the id as cacheKey without serializing, but I found
  /// duplicated keys. I don't think "hashable" works how I expected.
  ///     static cache = new Map<PlaceId, Place>();
  static cache = new Map<string, Place>();

  public static async fetchFromSerializedId(
    serializedId: string,
  ): Promise<Place> {
    const id = PlaceId.deserialize(serializedId);
    return PlaceStorage.fetchFromId(id);
  }

  public static async fetchFromId(id: PlaceId): Promise<Place> {
    const cacheKey = id.serialized();
    const cached = this.cache.get(cacheKey);

    if (cached) {
      return cached;
    }

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
    if (!feature) {
      if (!id.location) {
        // presumably if it was a gid, we'd hit something in pelias
        throw new Error(`missing location for id: ${id}`);
      }
      const place = Place.bareLocation(id.location);
      this.cache.set(cacheKey, place);
      return place;
    }

    const place = Place.fromFeature(id, feature);

    // if user clicked on location, keep that precise location
    if (id.location) {
      place.point = id.location;
    }

    this.cache.set(cacheKey, place);
    return place;
  }
}

type PlaceProperties = {
  countryCode?: string;
  name?: string;
  address?: string;
  phone?: string;
  website?: string;
  openingHours?: string;
};

/// Wrapper around a pelias response
export default class Place {
  id: PlaceId;
  point: LngLat;
  bbox?: LngLatBounds | undefined;
  countryCode?: string | undefined;
  public address?: string | null | undefined;
  name?: string | undefined;
  public phone?: string | undefined;
  public website?: string | undefined;
  public openingHours?: string | undefined;

  constructor(
    id: PlaceId,
    point: LngLat,
    bbox?: LngLatBounds,
    props: PlaceProperties = {},
  ) {
    this.id = id;
    this.point = point;
    this.bbox = bbox;
    this.countryCode = props.countryCode;
    this.name = props.name;
    this.address = props.address;
    this.phone = props.phone;
    this.website = props.website;
    this.openingHours = props.openingHours;
  }

  static fromFeature(id: PlaceId, feature: GeoJSON.Feature): Place {
    const geometry = feature.geometry as GeoJSON.Point;
    console.assert(geometry, 'no geometry found for feature', feature);
    console.assert(
      geometry.type == 'Point',
      'unexpected geometry found for feature',
      geometry,
    );
    const [lng, lat] = geometry.coordinates;
    console.assert(lng, 'missing lng');
    console.assert(lat, 'missing lat');
    const location = new LngLat(lng!, lat!);

    let bbox;
    if (feature.bbox?.length == 4) {
      bbox = LngLatBounds.convert(
        feature.bbox as [number, number, number, number],
      );
    } else {
      console.assert(
        !feature.bbox,
        'bbox was present, but had unexpected length',
        feature.bbox,
      );
    }

    const countryCode = feature.properties?.country_code;
    const address = localizeAddress(feature.properties);

    // This happens when searching for continent - e.g. "North America"
    if (
      feature.properties?.layer == 'continent' ||
      feature.properties?.layer == 'empire'
    ) {
      console.assert(
        !countryCode,
        'expecting continent to have no countryCode',
        feature,
      );
      console.assert(
        !address,
        'expecting continent to have no address',
        feature,
      );
    } else {
      console.assert(countryCode, 'no country code found for feature', feature);
      console.assert(address, 'no address found for feature', feature);
    }

    const name = feature.properties?.name;
    console.assert(name, 'no name found for feature', feature);

    // "addendum": {
    //   "osm": {
    //       "website": "https://www.adasbooks.com",
    //       "phone": "+1 206 322 1058",
    //       "opening_hours": "Su-Th 08:00-21:00; Fr-Sa 08:00-22:00"
    //   }
    // }
    const website = feature.properties?.addendum?.osm?.website;
    const phone = feature.properties?.addendum?.osm?.phone;
    const openingHours = feature.properties?.addendum?.osm?.opening_hours;

    const place = new Place(id, location, bbox, {
      countryCode,
      name,
      address,
      website,
      phone,
      openingHours,
    });
    return place;
  }

  static bareLocation(location: LngLat) {
    return new Place(PlaceId.location(location), location);
  }

  public serializedId(): string {
    return this.id.serialized();
  }

  public urlEncodedId(): string {
    return this.id.urlEncoded();
  }

  public preferredDistanceUnits(): DistanceUnits | undefined {
    const imperialDogs = ['US', 'GB', 'LR', 'MM'];
    if (!this.countryCode) {
      return undefined;
    }

    if (imperialDogs.includes(this.countryCode)) {
      return DistanceUnits.Miles;
    } else {
      return DistanceUnits.Kilometers;
    }
  }
}
