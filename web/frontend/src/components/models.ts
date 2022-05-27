export interface POI {
  key?: string;
  name?: string | null;
  address?: string | null;
  position?: LongLat;
  id?: number;
  type?: string | null;
}

export interface LongLat {
  long: number;
  lat: number;
}

export function poiDisplayName(poi: POI | undefined): string {
  if (poi?.name !== undefined) {
    return poi?.name ? poi?.name : '';
  }
  if (poi?.address) {
    return poi?.address;
  }
  return 'Dropped Pin';
}

export function canonicalizePoi(poi?: POI): string {
  if (poi?.id && poi?.type) {
    return `${poi.type}${poi.id}`;
  }
  if (poi?.position) {
    return `${poi.position?.long},${poi.position?.lat}`;
  }
  return '';
}

export async function decanonicalizePoi(
  poiString: string
): Promise<POI | undefined> {
  if (/([NWR][0-9]+)/.test(poiString)) {
    const response = await fetch(`/nominatim/lookup/${poiString}`);
    if (response.status != 200) {
      console.error(
        `Could not fetch POI data for ${poiString}. Is nominatim down?`
      );
      return;
    }
    const text = await response.text();
    const parser = new DOMParser();
    const xmlPoi = parser.parseFromString(text, 'text/xml');
    const placeTag = xmlPoi.getElementsByTagName('place').item(0);
    const position = {
      lat: parseFloat(
        placeTag?.attributes?.getNamedItem('lat')?.textContent as string
      ),
      long: parseFloat(
        placeTag?.attributes?.getNamedItem('lon')?.textContent as string
      ),
    };
    const houseNumber = xmlPoi
      .getElementsByTagName('house_number')
      .item(0)?.textContent;
    const clazz = placeTag?.attributes?.getNamedItem('class')
      ?.textContent as string;
    const road = xmlPoi.getElementsByTagName('road').item(0)?.textContent;
    const name =
      clazz !== 'place'
        ? xmlPoi.getElementsByTagName(clazz as string).item(0)?.textContent
        : undefined;
    const suburb = xmlPoi.getElementsByTagName('suburb').item(0)?.textContent;
    const city = xmlPoi.getElementsByTagName('city').item(0)?.textContent;

    const address = localizeAddress(houseNumber, road, suburb, city);

    return {
      name: name,
      address: address,
      position: position,
      id: parseInt(poiString.substring(1)),
      type: poiString.substring(0, 1),
    };
  } else if (/([0-9\.-]+,[0-9\.-]+)/.test(poiString)) {
    const longLat = poiString.split(',');
    return {
      position: {
        long: parseFloat(longLat[0]),
        lat: parseFloat(longLat[1]),
      },
    };
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
