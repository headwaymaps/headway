import { i18n } from 'src/i18n/lang';
import {
  ValhallaRoute,
  ValhallaRouteLegManeuver,
} from 'src/services/ValhallaAPI';
import { DistanceUnits, TravelMode } from 'src/utils/models';

export default class Route {
  durationSeconds: number;
  viaRoadsFormatted: string;
  preferredDistanceUnits: DistanceUnits;
  mode: TravelMode;
  valhallaRoute: ValhallaRoute;

  constructor(args: {
    durationSeconds: number;
    viaRoadsFormatted: string;
    distanceUnits: DistanceUnits;
    mode: TravelMode;
    valhallaRoute: ValhallaRoute;
  }) {
    this.durationSeconds = args.durationSeconds;
    this.viaRoadsFormatted = args.viaRoadsFormatted;
    this.preferredDistanceUnits = args.distanceUnits;
    this.mode = args.mode;
    this.valhallaRoute = args.valhallaRoute;
  }

  public static fromValhalla(
    route: ValhallaRoute,
    mode: TravelMode,
    distanceUnits: DistanceUnits,
  ): Route {
    console.assert(route.legs.length > 0, 'missing legs');
    const viaRoads = substantialRoadNames(route.legs[0]!.maneuvers, 3);
    return new Route({
      mode,
      valhallaRoute: route,
      durationSeconds: route.summary.time,
      viaRoadsFormatted: viaRoads.join(
        i18n.global.t('punctuation_list_seperator'),
      ),
      distanceUnits,
    });
  }
}

function substantialRoadNames(
  maneuvers: ValhallaRouteLegManeuver[],
  limit: number,
): string[] {
  const roadLengths = [];
  let cumulativeRoadLength = 0.0;
  for (const maneuver of maneuvers) {
    const length = maneuver.length;
    cumulativeRoadLength += length;
    if (maneuver.street_names) {
      const name = maneuver.street_names[0];
      console.assert(name, 'missing street name');
      roadLengths.push({ name: name!, length });
    }
  }
  roadLengths.sort((a, b) => b.length - a.length).slice(0, limit);

  // Don't include tiny segments in the description of the route
  const inclusionThreshold = cumulativeRoadLength / (limit + 1);
  let substantialRoads = roadLengths.filter(
    (r) => r.length > inclusionThreshold,
  );

  if (substantialRoads.length == 0) {
    console.assert(roadLengths.length > 0);
    substantialRoads = [roadLengths[0]!];
  }

  return substantialRoads.map((r) => r.name);
}
