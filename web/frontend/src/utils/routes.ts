export interface RouteLegManeuver {
  begin_shape_index: number;
  end_shape_index: number;
  street_names: string[];
  time: number;
  cost: number;
  length: number;
  instruction: string;
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
  timeSeconds: number;
  viaRoads: string[];
  timeFormatted: string;
  viaRoadsFormatted: string;
  lengthFormatted: string;
}

export function summarizeRoute(route: Route): ProcessedRouteSummary {
  const viaRoads = [];
  let cumulativeRoadCost = 0;
  for (let i = 0; i < 4; i++) {
    const roadCosts = costliestRoads(route.legs[0], viaRoads);
    const roads: string[] = [];
    roadCosts.forEach((roadTime: number, roadName: string) => {
      roads.push(roadName);
    });
    roads.sort((a: string, b: string) => {
      const aCost = roadCosts.get(a);
      const bCost = roadCosts.get(b);
      if (aCost && bCost) {
        return bCost - aCost;
      }
      return 0;
    });
    console.log(roadCosts);
    console.log(roads);
    const road = roads[0];
    const roadCost = roadCosts.get(road);
    if (!roadCost) {
      continue;
    }
    if (roadCost < 0.5 || roadCost < 0.25 * cumulativeRoadCost) {
      break;
    }
    cumulativeRoadCost += roadCost;
    viaRoads.push(road);
  }
  return {
    timeSeconds: route.summary.time,
    viaRoads: viaRoads,
    timeFormatted: formatTime(route.summary.time),
    viaRoadsFormatted: viaRoads.join(', '), // i18n
    lengthFormatted:
      route.summary.length.toFixed(1) +
      ' ' +
      route.units.replace('kilometers', 'km').replace('miles', 'mi'), // i18n
  };
}

function costliestRoads(
  leg: RouteLeg,
  ignoring: string[]
): Map<string, number> {
  const roadCosts = new Map<string, number>();

  console.log(ignoring);

  for (const manueverIndex in leg.maneuvers) {
    const maneuver = leg.maneuvers[manueverIndex];
    let mustIgnore = false;
    if (!maneuver.street_names) {
      continue;
    }
    for (const idx in maneuver.street_names) {
      const road = maneuver.street_names[idx];
      if (ignoring.indexOf(road) !== -1) {
        mustIgnore = true;
        break;
      }
    }
    if (mustIgnore) {
      continue;
    }
    for (const idx in maneuver.street_names) {
      const key = maneuver.street_names[idx];
      const oldCost = roadCosts.get(key);
      // Penalize long names slightly.
      const mult = 10000.0 / (10000.0 + key.length);
      if (oldCost) {
        roadCosts.set(key, oldCost + mult * maneuver.length);
      } else {
        roadCosts.set(key, mult * maneuver.length);
      }
    }
  }
  return roadCosts;
}

function formatTime(timeSeconds: number): string {
  const totalMinutes = Math.round(timeSeconds / 60);
  let timeString = '';
  if (totalMinutes < 1) {
    timeString = Math.round(timeSeconds) + ' seconds'; // i18n
  } else if (totalMinutes < 60) {
    timeString = totalMinutes + ' minutes'; // i18n
  } else {
    const days = Math.floor(totalMinutes / 60 / 24);
    const hours = Math.floor((totalMinutes - days * 24 * 60) / 60);
    const minutes = Math.round(totalMinutes - days * 24 * 60 - hours * 60);
    const timeStringComponents = [];
    if (days == 1) {
      timeStringComponents.push(days + ' day');
    } else if (days > 1) {
      timeStringComponents.push(days + ' days');
    }
    if (hours == 1) {
      timeStringComponents.push(hours + ' hour');
    } else if (hours > 1) {
      timeStringComponents.push(hours + ' hours');
    }
    if (minutes == 1) {
      timeStringComponents.push(minutes + ' minute');
    } else if (minutes > 1) {
      timeStringComponents.push(minutes + ' minutes');
    }
    timeString = timeStringComponents.join(', ');
  }
  return timeString;
}
