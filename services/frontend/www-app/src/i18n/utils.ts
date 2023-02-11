import Place from 'src/models/Place';
import { formatLngLatAsLatLng } from 'src/utils/format';

export function placeDisplayName(place: Place): string {
  if (place.name) {
    return place.name;
  }
  if (place.address) {
    return place.address;
  }
  return formatLngLatAsLatLng(place.point);
}
