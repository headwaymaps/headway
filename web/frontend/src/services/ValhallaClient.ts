import { formatDuration } from 'src/utils/format';
import { i18n } from 'src/i18n/lang';

export interface ValhallaRouteLegManeuver {
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

export interface ValhallaRouteSummary {
  time: number;
  length: number;
  min_lat: number;
  min_lon: number;
  max_lat: number;
  max_lon: number;
}

export interface ValhallaRouteLeg {
  maneuvers: ValhallaRouteLegManeuver[];
  shape: string;
}

export interface ValhallaRoute {
  legs: ValhallaRouteLeg[];
  summary: ValhallaRouteSummary;
  units: string;
}

export interface ProcessedRouteSummary {
  durationSeconds: number;
  durationFormatted: string;
  viaRoadsFormatted: string;
  lengthFormatted: string;
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

import { POI, DistanceUnits } from 'src/utils/models';

export type CacheableMode = 'walk' | 'bicycle' | 'car';

function modeToCostingModel(mode: CacheableMode): string {
  switch (mode) {
    case 'walk':
      return 'pedestrian';
    case 'bicycle':
      return 'bicycle';
    case 'car':
      return 'auto';
  }
}

export async function getRoutes(
  from: POI,
  to: POI,
  mode: CacheableMode,
  units?: DistanceUnits
): Promise<[ValhallaRoute, ProcessedRouteSummary][]> {
  if (!from.position || !to.position) {
    console.error("Can't request without fully specified endpoints");
    return [];
  }

  type RouteRequest = {
    locations: Array<{ lat: number; lon: number }>;
    costing: string;
    alternates: number;
    units?: DistanceUnits;
  };
  const requestObject: RouteRequest = {
    locations: [
      {
        lat: from.position.lat,
        lon: from.position.long,
      },
      {
        lat: to.position.lat,
        lon: to.position.long,
      },
    ],
    costing: modeToCostingModel(mode),
    alternates: 3,
  };
  if (units) {
    requestObject.units = units;
  }
  const response = await fetch(
    `/valhalla/route?json=${JSON.stringify(requestObject)}`
  );
  if (response.status !== 200) {
    console.error('Valhalla response gave error: ' + response.status);
    return [];
  }
  const responseJson = await response.json();
  const routes: [ValhallaRoute, ProcessedRouteSummary][] = [];
  const route = responseJson.trip as ValhallaRoute;
  if (route) {
    routes.push([route, summarizeRoute(route)]);
  }
  for (const altIdx in responseJson.alternates) {
    const route = responseJson.alternates[altIdx].trip as ValhallaRoute;
    if (route) {
      routes.push([route, summarizeRoute(route)]);
    }
  }
  return routes;
}

export function summarizeRoute(route: ValhallaRoute): ProcessedRouteSummary {
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
  maneuvers: ValhallaRouteLegManeuver[],
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
