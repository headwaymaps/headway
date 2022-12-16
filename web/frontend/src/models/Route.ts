import { i18n } from 'src/i18n/lang';
import {
  ValhallaRoute,
  getRoutes as getValhallaRoutes,
  CacheableMode,
  ValhallaRouteLegManeuver,
  ValhallaError,
  ValhallaErrorCode,
} from 'src/services/ValhallaClient';
import { formatDuration } from 'src/utils/format';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { decodeValhallaPath } from 'src/third_party/decodePath';
import { LngLatBounds, LngLat, LineLayerSpecification } from 'maplibre-gl';
import Trip, { TripLeg } from './Trip';
import { Err, Ok, Result } from 'src/utils/Result';

export enum RouteErrorCode {
  Other,
  UnsupportedArea,
}

export class RouteError {
  errorCode: RouteErrorCode;
  message: string;

  constructor(errorCode: RouteErrorCode, message: string) {
    this.errorCode = errorCode;
    this.message = message;
  }

  static fromValhalla(vError: ValhallaError): RouteError {
    switch (vError.error_code) {
      case ValhallaErrorCode.UnsupportedArea: {
        return {
          errorCode: RouteErrorCode.UnsupportedArea,
          message: vError.error,
        };
      }
    }
  }
}

export default class Route implements Trip {
  durationSeconds: number;
  durationFormatted: string;
  viaRoadsFormatted: string;
  lengthFormatted: string;
  mode: TravelMode;
  valhallaRoute: ValhallaRoute;

  constructor(args: {
    durationSeconds: number;
    durationFormatted: string;
    viaRoadsFormatted: string;
    lengthFormatted: string;
    mode: TravelMode;
    valhallaRoute: ValhallaRoute;
  }) {
    this.durationSeconds = args.durationSeconds;
    this.durationFormatted = args.durationFormatted;
    this.viaRoadsFormatted = args.viaRoadsFormatted;
    this.lengthFormatted = args.lengthFormatted;
    this.mode = args.mode;
    this.valhallaRoute = args.valhallaRoute;
  }

  public get bounds(): LngLatBounds {
    const summary = this.valhallaRoute.summary;
    return new LngLatBounds(
      new LngLat(summary.min_lon, summary.min_lat),
      new LngLat(summary.max_lon, summary.max_lat)
    );
  }

  public get legs(): TripLeg[] {
    return this.valhallaRoute.legs.map((vLeg): TripLeg => {
      return {
        geometry(): GeoJSON.LineString {
          const points: [number, number][] = [];
          decodeValhallaPath(vLeg.shape, 6).forEach((point) => {
            points.push([point[1], point[0]]);
          });
          return {
            type: 'LineString',
            coordinates: points,
          };
        },
        paintStyle(active: boolean): LineLayerSpecification['paint'] {
          if (active) {
            return {
              'line-color': '#1976D2',
              'line-width': 6,
            };
          } else {
            return {
              'line-color': '#777',
              'line-width': 4,
              'line-dasharray': [0.5, 2],
            };
          }
        },
      };
    });
  }

  public static async getRoutes(
    from: LngLat,
    to: LngLat,
    mode: CacheableMode,
    units?: DistanceUnits
  ): Promise<Result<Route[], RouteError>> {
    const result = await getValhallaRoutes(from, to, mode, units);
    if (result.ok) {
      const valhallaRoutes = result.value;
      // This is only safe as long as CacheableMode is a subset of TravelMode
      return Ok(valhallaRoutes.map((r) => fromValhalla(r, mode as TravelMode)));
    } else {
      const valhallaError = result.error;
      return Err(RouteError.fromValhalla(valhallaError));
    }
  }
}

function fromValhalla(route: ValhallaRoute, mode: TravelMode): Route {
  const viaRoads = substantialRoadNames(route.legs[0].maneuvers, 3);
  return new Route({
    mode,
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
