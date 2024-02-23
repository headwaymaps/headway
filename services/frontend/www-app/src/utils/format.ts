import { LngLat, LngLatLike } from 'maplibre-gl';
import { i18n } from 'src/i18n/lang';
import { DistanceUnits } from './models';

export function formatDuration(
  durationSeconds: number,
  format: undefined | 'shortform' = undefined,
): string {
  let formatModifier = '';
  if (format) {
    formatModifier = '_' + format;
  }
  const totalMinutes = Math.round(durationSeconds / 60);
  let timeString = '';
  if (totalMinutes < 1) {
    timeString = i18n.global.t(`times${formatModifier}.$n_seconds`, {
      n: Math.round(durationSeconds),
    });
  } else if (totalMinutes < 60) {
    timeString = i18n.global.t(`times${formatModifier}.$n_minutes`, {
      n: totalMinutes,
    });
  } else {
    const days = Math.floor(totalMinutes / 60 / 24);
    const hours = Math.floor((totalMinutes - days * 24 * 60) / 60);
    const minutes = Math.round(totalMinutes - days * 24 * 60 - hours * 60);
    const timeStringComponents = [];
    if (days == 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_day`, { n: days }),
      );
    } else if (days > 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_days`, { n: days }),
      );
    }
    if (hours == 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_hour`, { n: hours }),
      );
    } else if (hours > 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_hours`, { n: hours }),
      );
    }
    if (minutes == 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_minute`, { n: minutes }),
      );
    } else if (minutes > 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_minutes`, { n: minutes }),
      );
    }
    if (format == 'shortform') {
      timeString = timeStringComponents.join(' ');
    } else {
      timeString = timeStringComponents.join(
        i18n.global.t('punctuation_list_seperator'),
      );
    }
  }
  return timeString;
}

export function formatLngLatAsLatLng(point: LngLatLike): string {
  const lngLat = LngLat.convert(point);
  const northOrSouth = lngLat.lat < 0 ? 'S' : 'N';
  const eastOrWest = lngLat.lng < 0 ? 'W' : 'E';

  const fmt = (f: number) => Math.abs(f).toFixed(6);

  return `${fmt(lngLat.lat)}°${northOrSouth}, ${fmt(lngLat.lng)}°${eastOrWest}`;
}

export function formatTime(dateArgs: number | string | Date): string {
  return new Date(dateArgs).toLocaleTimeString([], { timeStyle: 'short' });
}

/**
 * Very concise time formatting
 *
 * 10:00 AM -> 10 AM
 * 10:01 AM -> 10:01 AM (unchanged when minutes are non-zero)
 */
export function formatTimeTruncatingEmptyMinutes(
  dateArgs: number | string | Date,
): string {
  // This is admittedly a hack, and probably doesn't won't improve some
  // localizations, but I don't know of a better way that doesn't break other
  // localizations.
  return formatTime(dateArgs).replace(':00', '');
}

export function dayOfWeek(dateArgs: number | string | Date): string {
  return new Date(dateArgs).toLocaleString([], { weekday: 'short' });
}

export function metersToMiles(meters: number): number {
  return meters / 621.37119;
}

export function formatMeters(
  meters: number,
  outputUnits: DistanceUnits,
  precision = 1,
): string {
  let distance;
  switch (outputUnits) {
    case DistanceUnits.Kilometers:
      distance = meters / 1000;
      break;
    case DistanceUnits.Miles:
      distance = metersToMiles(meters);
      break;
  }

  const rounded = distance.toFixed(precision);
  if (outputUnits == DistanceUnits.Kilometers) {
    return `${rounded} ${i18n.global.t('shortened_distances.kilometers')}`;
  } else {
    return `${rounded} ${i18n.global.t('shortened_distances.miles')}`;
  }
}
