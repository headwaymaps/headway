import { i18n } from 'src/i18n/lang';
import { DistanceUnits } from './models';

export interface RouteLegManeuver {
  begin_shape_index: number;
  end_shape_index: number;
  street_names?: string[];
  time: number;
  cost: number;
  length: number;
  instruction: string;
  verbal_post_transition_instruction: string;
  type: number;
}

export interface RouteSummary {
  time: number;
  length: number;
  min_lat: number;
  min_lon: number;
  max_lat: number;
  max_lon: number;
}

export interface RouteLeg {
  maneuvers: RouteLegManeuver[];
  shape: string;
}

export interface Route {
  legs: RouteLeg[];
  summary: RouteSummary;
  units: string;
}

export interface ProcessedRouteSummary {
  durationSeconds: number;
  durationFormatted: string;
  viaRoadsFormatted: string;
  lengthFormatted: string;
}

export function summarizeRoute(route: Route): ProcessedRouteSummary {
  const viaRoads = substantialRoadNames(route.legs[0].maneuvers, 3);
  return {
    durationSeconds: route.summary.time,
    durationFormatted: formatDuration(route.summary.time, 'shortform'),
    viaRoadsFormatted: viaRoads.join(
      i18n.global.t('punctuation_list_seperator')
    ),
    lengthFormatted:
      route.summary.length.toFixed(1) +
      ' ' +
      route.units
        .replace('kilometers', i18n.global.t('shortened_distances.kilometers'))
        .replace('miles', i18n.global.t('shortened_distances.miles')),
  };
}

function substantialRoadNames(
  maneuvers: RouteLegManeuver[],
  limit: number
): string[] {
  const roadLengths = [];
  let cumulativeRoadLength = 0.0;
  for (const maneuver of maneuvers) {
    const length = maneuver.length;
    cumulativeRoadLength += length;
    if (maneuver.street_names) {
      const name = maneuver.street_names[0];
      roadLengths.push({ name, length });
    }
  }
  roadLengths.sort((a, b) => b.length - a.length).slice(0, limit);

  // Don't include tiny segments in the description of the route
  const inclusionThreshold = cumulativeRoadLength / (limit + 1);
  let substantialRoads = roadLengths.filter(
    (r) => r.length > inclusionThreshold
  );

  if (substantialRoads.length == 0) {
    substantialRoads = [roadLengths[0]];
  }

  return substantialRoads.map((r) => r.name);
}

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

export function valhallaTypeToIcon(type: number) {
  switch (type) {
    case 1:
    case 2:
    case 3:
      return 'straight';
    case 4:
    case 5:
    case 6:
      return 'place';
    case 7:
    case 8:
      return 'straight';
    case 9:
      return 'turn_slight_right';
    case 10:
      return 'turn_right';
    case 11:
      return 'turn_sharp_right';
    case 12:
      return 'u_turn_right';
    case 13:
      return 'u_turn_left';
    case 14:
      return 'turn_sharp_left';
    case 15:
      return 'turn_left';
    case 16:
      return 'turn_slight_left';
    case 17:
      return 'straight';
    case 18:
      return 'turn_slight_right';
    case 19:
      return 'turn_slight_left';
    case 20:
      return 'turn_slight_right';
    case 21:
      return 'turn_slight_left';
    case 22:
      return 'straight';
    case 23:
      return 'turn_slight_right';
    case 24:
      return 'turn_slight_left';
    case 25:
      return 'merge';
    case 26:
      return 'login';
    case 27:
      return 'logout';
    case 28:
      return 'login';
    case 29:
      return 'logout';
  }
  return '';
}
