import { i18n } from 'src/i18n/lang';
import { DistanceUnits } from './models';

export function formatDuration(
  durationSeconds: number,
  format: undefined | 'shortform' = undefined
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
        i18n.global.t(`times${formatModifier}.$n_day`, { n: days })
      );
    } else if (days > 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_days`, { n: days })
      );
    }
    if (hours == 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_hour`, { n: hours })
      );
    } else if (hours > 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_hours`, { n: hours })
      );
    }
    if (minutes == 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_minute`, { n: minutes })
      );
    } else if (minutes > 1) {
      timeStringComponents.push(
        i18n.global.t(`times${formatModifier}.$n_minutes`, { n: minutes })
      );
    }
    if (format == 'shortform') {
      timeString = timeStringComponents.join(' ');
    } else {
      timeString = timeStringComponents.join(
        i18n.global.t('punctuation_list_seperator')
      );
    }
  }
  return timeString;
}

export function formatTime(millis: number): string {
  return new Date(millis).toLocaleTimeString([], { timeStyle: 'short' });
}

export function kilometersToMiles(kilometers: number): number {
  return kilometers * 0.62137119;
}

export function formatDistance(
  distance: number,
  units: DistanceUnits,
  precision = 1
): string {
  const rounded = distance.toFixed(precision);
  if (units == DistanceUnits.Kilometers) {
    return `${rounded} ${i18n.global.t('shortened_distances.kilometers')}`;
  } else {
    return `${rounded} ${i18n.global.t('shortened_distances.miles')}`;
  }
}
