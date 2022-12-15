import { i18n } from './lang';
import { POI } from 'src/utils/models';

export function poiDisplayName(poi?: POI): string {
  if (poi?.name) {
    return poi?.name;
  }
  if (poi?.address) {
    return poi?.address;
  }
  return i18n.global.t('dropped_pin');
}
