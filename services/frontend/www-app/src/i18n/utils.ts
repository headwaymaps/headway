import { i18n } from './lang';
import Place from 'src/models/Place';

export function placeDisplayName(place: Place): string {
  if (place.name) {
    return place.name;
  }
  if (place.address) {
    return place.address;
  }
  return i18n.global.t('dropped_pin');
}
