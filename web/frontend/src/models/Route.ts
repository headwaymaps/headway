import { i18n } from 'src/i18n/lang';
import {
  ValhallaRoute,
  getRoutes as getValhallaRoutes,
  CacheableMode,
  ValhallaRouteLegManeuver,
} from 'src/services/ValhallaClient';
import { formatDuration } from 'src/utils/format';
import { POI, DistanceUnits } from 'src/utils/models';
import { decodeValhallaPath } from 'src/third_party/decodePath';
import { LngLatBounds, LngLat } from 'maplibre-gl';

export default class Route {
  durationSeconds: number;
  durationFormatted: string;
  viaRoadsFormatted: string;
  lengthFormatted: string;
  valhallaRoute: ValhallaRoute;
  constructor(args: {
    durationSeconds: number;
    durationFormatted: string;
    viaRoadsFormatted: string;
    lengthFormatted: string;
    valhallaRoute: ValhallaRoute;
  }) {
    this.durationSeconds = args.durationSeconds;
    this.durationFormatted = args.durationFormatted;
    this.viaRoadsFormatted = args.viaRoadsFormatted;
    this.lengthFormatted = args.lengthFormatted;
    this.valhallaRoute = args.valhallaRoute;
  }

  public geometry(): GeoJSON.Geometry {
    const points: [number, number][] = [];
    decodeValhallaPath(this.valhallaRoute.legs[0].shape, 6).forEach((point) => {
      points.push([point[1], point[0]]);
    });
    return {
      type: 'LineString',
      coordinates: points,
    };
  }

  public get bounds(): LngLatBounds {
    const summary = this.valhallaRoute.summary;
    return new LngLatBounds(
      new LngLat(summary.min_lon, summary.min_lat),
      new LngLat(summary.max_lon, summary.max_lat)
    );
  }

  public static async getRoutes(
    from: POI,
    to: POI,
    mode: CacheableMode,
    units?: DistanceUnits
  ): Promise<Route[]> {
    const valhallaRoutes = await getValhallaRoutes(from, to, mode, units);
    return valhallaRoutes.map(fromValhalla);
  }
}

function fromValhalla(route: ValhallaRoute): Route {
  const viaRoads = substantialRoadNames(route.legs[0].maneuvers, 3);
  return new Route({
    valhallaRoute: route,
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
  });
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
