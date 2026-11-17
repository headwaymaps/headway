import Trip from './Trip';
import type {
  BikeRoutePreference,
  TravelmuxComparison,
} from 'src/services/TravelmuxClient';

/// A router to compare, as listed in the compare stack's compare.json.
export type ComparedRouter = {
  name: string;
  /// What travelmux calls it, like `valhalla-headway`.
  routerId: string;
  /// Where to read about it, like its commit.
  link: string;
};

/// One router's plan for a route, whose time can differ from another router's on the same streets.
export type RouterPlan = {
  router: string;
  durationSeconds: number;
  routePreferences: BikeRoutePreference[];
};

export type ComparedRoute = {
  /// Without route preferences, which belong to each router's plan instead.
  trip: Trip;
  plans: RouterPlan[];
};

/// The routes several routers planned for one trip, as travelmux merged them.
export default class ComparedRoutes {
  readonly routes: ComparedRoute[];

  /// `routerName` labels a router by its id.
  constructor(
    trips: Trip[],
    comparison: TravelmuxComparison,
    routerName: (routerId: string) => string,
  ) {
    this.routes = comparison.routes.map(({ itineraries }) => {
      const routeTrips = itineraries.map((i) => trips[i]!);
      const first = routeTrips[0]!;
      return {
        trip: new Trip(
          { ...first.raw, routePreferences: undefined },
          first.preferredDistanceUnits,
        ),
        plans: routeTrips.map((trip) => ({
          router: routerName(trip.raw.routerId),
          durationSeconds: trip.raw.durationSeconds,
          routePreferences: trip.raw.routePreferences ?? [],
        })),
      };
    });
  }
}
