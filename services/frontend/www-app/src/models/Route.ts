import { i18n } from 'src/i18n/lang';
import {
  ValhallaRoute,
  ValhallaRouteLegManeuver,
  ValhallaError,
  ValhallaErrorCode,
  ValhallaRouteLeg,
  ValhallaTravelMode,
} from 'src/services/ValhallaClient';
import { formatDuration } from 'src/utils/format';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { decodePolyline } from 'src/third_party/decodePath';
import { LngLatBounds, LngLat, LineLayerSpecification } from 'maplibre-gl';
import Trip, { LineStyles, TripLeg } from './Trip';

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
  preferredDistanceUnits: DistanceUnits;
  distanceFormatted: string;
  mode: TravelMode;
  valhallaRoute: ValhallaRoute;

  constructor(args: {
    durationSeconds: number;
    durationFormatted: string;
    viaRoadsFormatted: string;
    distanceFormatted: string;
    distanceUnits: DistanceUnits;
    mode: TravelMode;
    valhallaRoute: ValhallaRoute;
  }) {
    this.durationSeconds = args.durationSeconds;
    this.durationFormatted = args.durationFormatted;
    this.viaRoadsFormatted = args.viaRoadsFormatted;
    this.distanceFormatted = args.distanceFormatted;
    this.preferredDistanceUnits = args.distanceUnits;
    this.mode = args.mode;
    this.valhallaRoute = args.valhallaRoute;
  }

  public get bounds(): LngLatBounds {
    const summary = this.valhallaRoute.summary;
    return new LngLatBounds(
      new LngLat(summary.min_lon, summary.min_lat),
      new LngLat(summary.max_lon, summary.max_lat),
    );
  }

  public get legs(): TripLeg[] {
    return this.valhallaRoute.legs.map((vLeg: ValhallaRouteLeg): TripLeg => {
      return {
        get geometry(): GeoJSON.LineString {
          const points: [number, number][] = [];
          decodePolyline(vLeg.shape, 6, true).forEach((point) => {
            points.push([point[1], point[0]]);
          });
          return {
            type: 'LineString',
            coordinates: points,
          };
        },
        get start(): LngLat {
          const coordinates = this.geometry.coordinates;
          return new LngLat(coordinates[0][0], coordinates[0][1]);
        },
        mode: this.mode,
        paintStyle(active: boolean): LineLayerSpecification['paint'] {
          if (active) {
            const firstManeuver = vLeg.maneuvers[0];
            console.assert(
              firstManeuver,
              'expected at least one maneuver to be set',
            );
            const isWalking =
              firstManeuver?.travel_mode == ValhallaTravelMode.Walk;

            // Currently anyway valhalla legs are single-mode. If we change that
            // we'll have to revisit this logic which assumes that the first
            // maneuver sets the mode for the entire leg.
            if (isWalking) {
              return LineStyles.walkingActive;
            } else {
              return LineStyles.active;
            }
          } else {
            return LineStyles.inactive;
          }
        },
      };
    });
  }

  public static fromValhalla(
    route: ValhallaRoute,
    mode: TravelMode,
    distanceUnits: DistanceUnits,
  ): Route {
    const viaRoads = substantialRoadNames(route.legs[0].maneuvers, 3);
    return new Route({
      mode,
      valhallaRoute: route,
      durationSeconds: route.summary.time,
      durationFormatted: formatDuration(route.summary.time, 'shortform'),
      viaRoadsFormatted: viaRoads.join(
        i18n.global.t('punctuation_list_seperator'),
      ),
      distanceFormatted: 'TODO: is valhalla distance used?',
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
      roadLengths.push({ name, length });
    }
  }
  roadLengths.sort((a, b) => b.length - a.length).slice(0, limit);

  // Don't include tiny segments in the description of the route
  const inclusionThreshold = cumulativeRoadLength / (limit + 1);
  let substantialRoads = roadLengths.filter(
    (r) => r.length > inclusionThreshold,
  );

  if (substantialRoads.length == 0) {
    substantialRoads = [roadLengths[0]];
  }

  return substantialRoads.map((r) => r.name);
}
