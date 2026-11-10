import Trip from './Trip';
import type {
  BikeRoutePreference,
  TravelmuxRouter,
} from 'src/services/TravelmuxClient';

/// A router to compare, as listed in the compare stack's compare.json.
export type ComparedRouter = {
  name: string;
  /// Where its travelmux is served, like `/travelmux/main`.
  travelmux: string;
  /// Where to read about it, like its commit.
  link: string;
  /// Which of its travelmux's routers to plan with, if not whichever suits the trip.
  router?: TravelmuxRouter;
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

/// The routes several routers planned for one trip, merged where they took exactly the same streets.
export default class ComparedRoutes {
  readonly routes: ComparedRoute[] = [];

  /// Adds `trips`, crediting `router` on any route it shares with another router.
  add(router: string, trips: Trip[]) {
    for (const trip of trips) {
      const plan: RouterPlan = {
        router,
        durationSeconds: trip.raw.durationSeconds,
        routePreferences: trip.raw.routePreferences ?? [],
      };
      const geometry = geometryKey(trip);
      const shared = this.routes.find(
        (route) =>
          !route.plans.some((plan) => plan.router === router) &&
          geometryKey(route.trip) === geometry,
      );
      if (shared) {
        shared.plans.push(plan);
      } else {
        this.routes.push({
          trip: new Trip(
            { ...trip.raw, routePreferences: undefined },
            trip.preferredDistanceUnits,
          ),
          plans: [plan],
        });
      }
    }
  }
}

function geometryKey(trip: Trip): string {
  return trip.raw.legs.map((leg) => leg.geometry).join('|');
}
