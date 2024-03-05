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
  console.assert(dateArgs !== undefined);
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

export function formatDistance(
  inputDistance: number,
  inputUnits: DistanceUnits,
  outputUnits: DistanceUnits,
  precision = 1,
): string {
  console.assert(inputUnits, 'missing input distance units');
  console.assert(outputUnits, 'missing output distance units');

  let distance;

  if (inputUnits == outputUnits) {
    distance = inputDistance;
  } else if (
    inputUnits == DistanceUnits.Miles &&
    outputUnits == DistanceUnits.Kilometers
  ) {
    distance = inputDistance * 1.60934;
  } else if (
    inputUnits == DistanceUnits.Miles &&
    outputUnits == DistanceUnits.Meters
  ) {
    distance = inputDistance * 1609.34;
  } else if (
    inputUnits == DistanceUnits.Kilometers &&
    outputUnits == DistanceUnits.Miles
  ) {
    distance = inputDistance * 0.6213727366;
  } else if (
    inputUnits == DistanceUnits.Kilometers &&
    outputUnits == DistanceUnits.Meters
  ) {
    distance = inputDistance * 1000;
  } else if (
    inputUnits == DistanceUnits.Meters &&
    outputUnits == DistanceUnits.Kilometers
  ) {
    distance = inputDistance * 0.001;
  } else if (
    inputUnits == DistanceUnits.Meters &&
    outputUnits == DistanceUnits.Miles
  ) {
    distance = inputDistance * 0.0006213727366;
  } else {
    console.assert(
      false,
      'unhandled case: ' + inputUnits + ' -> ' + outputUnits,
    );
    distance = inputDistance;
  }

  const rounded = distance.toFixed(precision);
  if (outputUnits == DistanceUnits.Kilometers) {
    return `${rounded} ${i18n.global.t('shortened_distances.kilometers')}`;
  } else if (outputUnits == DistanceUnits.Meters) {
    return `${rounded} ${i18n.global.t('shortened_distances.meters')}`;
  } else if (outputUnits == DistanceUnits.Miles) {
    return `${rounded} ${i18n.global.t('shortened_distances.miles')}`;
  } else {
    console.assert(`unhandled output units: ${outputUnits}`);
    return `${rounded}`;
  }
}
