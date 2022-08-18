export interface POI {
  key?: string;
  name?: string | null;
  address?: string | null;
  position?: LongLat;
  gid?: string;
}

export interface LongLat {
  long: number;
  lat: number;
}

export function poiDisplayName(poi: POI | undefined): string {
  if (poi?.name) {
    return poi?.name;
  }
  if (poi?.address) {
    return poi?.address;
  }
  return 'Dropped Pin';
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
  poiString: string
): Promise<POI | undefined> {
  if (/([0-9\.-]+,[0-9\.-]+)/.test(poiString)) {
    const longLat = poiString.split(',');
    return {
      position: {
        long: parseFloat(longLat[0]),
        lat: parseFloat(longLat[1]),
      },
    };
  } else {
    console.log(`decanonicalize ${poiString}`)
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
      const address = localizeAddress(
        feature.properties.housenumber,
        feature.properties.street,
        feature.properties.locality,
        feature.properties.city
      );

      const coordinates = feature?.geometry?.coordinates;
      const position: LongLat | undefined = coordinates
        ? { long: coordinates[0], lat: coordinates[1] }
        : undefined;
      return {
        name: feature.properties.name,
        address: address,
        key: feature.properties.osm_id,
        position: position,
        gid: feature?.properties?.gid,
      };
    }
    return undefined;
  }
}

// FIXME: this is US-only, if we get international users it must be expanded.
export function localizeAddress(
  houseNumber: string | undefined | null,
  road: string | undefined | null,
  neighborhood: string | undefined | null,
  city: string | undefined | null
) {
  if (houseNumber && road && neighborhood && city) {
    return `${houseNumber} ${road}, ${neighborhood}, ${city}`;
  }
  if (houseNumber && road && city) {
    return `${houseNumber} ${road}, ${city}`;
  }
  if (houseNumber && road) {
    return `${houseNumber} ${road}`;
  }
  if (road) {
    return `${road}`;
  }
  if (neighborhood && city) {
    return `${neighborhood}, ${city}`;
  }
  if (city) {
    return `${city}`;
  }
  return undefined;
}
