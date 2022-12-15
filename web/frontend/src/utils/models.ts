import addressFormatter from '@fragaria/address-formatter';
import { LngLatBoundsLike } from 'maplibre-gl';
import { LongLat } from './geomath';

const addressKeys = [
  'archipelago',
  'city',
  // 'continent',
  'country',
  'countryCode',
  'county',
  'hamlet',
  'house',
  'houseNumber',
  'island',
  'municipality',
  'neighbourhood',
  'postalCity',
  'postcode',
  'region',
  'road',
  'state',
  'stateDistrict',
  'village',
  'allotments',
  'borough',
  'building',
  'cityBlock',
  'cityDistrict',
  'commercial',
  'countryName',
  'countyCode',
  'croft',
  'department',
  'district',
  'farmland',
  'footway',
  'housenumber',
  'houses',
  'industrial',
  'isolatedDwelling',
  'localAdministrativeArea',
  'locality',
  'partialPostcode',
  'path',
  'pedestrian',
  'place',
  'postcode',
  'province',
  'publicBuilding',
  'quarter',
  'residential',
  'roadReference',
  'roadReferenceIntl',
  'square',
  'stateCode',
  'street',
  'streetName',
  'streetNumber',
  'subcounty',
  'subdistrict',
  'subdivision',
  'suburb',
  'town',
  'township',
  'ward',
];

export interface POI {
  key?: string;
  name?: string | null;
  address?: string | null;
  position?: LongLat;
  bbox?: LngLatBoundsLike;
  gid?: string;
}

export function canonicalizePoi(poi?: POI): string {
  if (poi?.gid) {
    return poi.gid;
  }
  if (poi?.position) {
    return `${poi.position?.long},${poi.position?.lat}`;
  }
  return '';
}

export function encodePoi(poi?: POI): string {
  return encodeURIComponent(canonicalizePoi(poi));
}

export async function decanonicalizePoi(
  poiStringRaw: string
): Promise<POI | undefined> {
  if (poiStringRaw == '_') {
    return undefined;
  }
  const poiString = decodeURIComponent(poiStringRaw);
  if (/([0-9\.-]+,[0-9\.-]+)/.test(poiString)) {
    const longLat = poiString.split(',');
    return {
      position: {
        long: parseFloat(longLat[0]),
        lat: parseFloat(longLat[1]),
      },
    };
  } else {
    const response = await fetch(`/pelias/v1/place?ids=${poiString}`);
    if (response.status != 200) {
      console.error(
        `Could not fetch POI data for ${poiString}. Is pelias down?`
      );
      return;
    }

    const results = await response.json();
    if (results.features.length > 0) {
      const feature = results.features[0];
      const address = localizeAddress(feature.properties);

      const coordinates = feature?.geometry?.coordinates;
      const position: LongLat | undefined = coordinates
        ? { long: coordinates[0], lat: coordinates[1] }
        : undefined;
      return {
        name: feature.properties.name,
        address: address,
        key: feature.properties.osm_id,
        position: position,
        bbox: feature.bbox,
        gid: feature?.properties?.gid,
      };
    }
    return undefined;
  }
}

export async function mapFeatureToPoi(
  feature: GeoJSON.Feature
): Promise<POI | undefined> {
  feature.geometry;
  const pointGeometry = feature.geometry as GeoJSON.Point;
  if (!pointGeometry) {
    console.error(
      "Geometry is not a point and Headway doesn't handle that yet"
    );
    return;
  }
  const lng = pointGeometry.coordinates[0];
  const lat = pointGeometry.coordinates[1];
  if (!lat || !lng) {
    console.error(
      `Could not reverse geocode ${JSON.stringify(
        feature
      )}. Unsupported geometry.`
    );
  }
  const response = await fetch(
    `/pelias/v1/reverse?point.lat=${lat}&point.lon=${lng}&boundary.circle.radius=0.1&sources=osm`
  );
  if (response.status != 200) {
    console.error(
      `Could not reverse ${JSON.stringify(feature)}. Is pelias down?`
    );
    return;
  }

  const results = await response.json();
  for (const id in results.features) {
    if (results.features[id]?.properties?.name !== feature?.properties?.name) {
      continue;
    }
    return decanonicalizePoi(results.features[id].properties.gid);
  }
  return undefined;
}

// eslint-disable-next-line
export function localizeAddress(properties: any, oneLine = true): string {
  // eslint-disable-next-line
  let addressProperties: any = {};
  for (const [key, value] of Object.entries(properties)) {
    if (key === 'region') {
      // This looks like a localization bug but pelias aliases state, province, etc to region which the address formatter doesn't expect. Alias them back.
      addressProperties['state'] = value;
    } else if (addressKeys.includes(key)) {
      addressProperties[key] = value;
    }
  }
  const address = addressFormatter.format(addressProperties, {
    abbreviate: true,
    output: 'string',
    countryCode: properties.country_code,
    appendCountry: false,
  });
  if (oneLine) {
    return address.trim().replaceAll('\n', ', ');
  }
  return address;
}

export function isDense(): boolean {
  return window.innerWidth <= 800;
}

export enum DistanceUnits {
  Miles = 'miles',
  Kilometers = 'kilometers',
}

export enum TravelMode {
  Bike = 'bicycle',
  Walk = 'walk',
  Drive = 'car',
  Transit = 'transit',
}
