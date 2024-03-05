import addressFormatter from '@fragaria/address-formatter';
import Place, { PlaceId, PlaceStorage } from 'src/models/Place';

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

export async function mapFeatureToPlace(
  feature: GeoJSON.Feature,
): Promise<Place | undefined> {
  const pointGeometry = feature.geometry as GeoJSON.Point;
  if (!pointGeometry) {
    console.error(
      "Geometry is not a point and Headway doesn't handle that yet",
    );
    return;
  }
  const lng = pointGeometry.coordinates[0];
  const lat = pointGeometry.coordinates[1];
  if (!lat || !lng) {
    console.error(
      `Could not reverse geocode ${JSON.stringify(
        feature,
      )}. Unsupported geometry.`,
    );
  }
  const response = await fetch(
    `/pelias/v1/reverse?point.lat=${lat}&point.lon=${lng}&boundary.circle.radius=0.1&sources=osm`,
  );
  if (response.status != 200) {
    console.error(
      `Could not reverse ${JSON.stringify(feature)}. Is pelias down?`,
    );
    return;
  }

  const results = await response.json();
  for (const id in results.features) {
    if (results.features[id]?.properties?.name !== feature?.properties?.name) {
      continue;
    }
    const gid = PlaceId.gid(results.features[id].properties.gid);
    return await PlaceStorage.fetchFromId(gid);
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
  Meters = 'meters',
}

export enum TravelMode {
  Bike = 'bicycle',
  Walk = 'walk',
  Drive = 'car',
  Transit = 'transit',
}
