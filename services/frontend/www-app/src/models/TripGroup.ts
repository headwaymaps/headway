import Trip, { RouteBadge } from 'src/models/Trip';

/// Departures of what is, for the rider, the same trip: one row in the list of options.
export default class TripGroup {
  /// In departure order.
  readonly trips: Trip[];

  private constructor(trips: Trip[]) {
    this.trips = trips;
  }

  static readonly maxDepartures = 4;

  /// Groups keep the order their first trip arrived in. A trip without a key, like an all-walking
  /// one, is always its own group. Each keeps only its next few departures after `now`.
  static group(trips: Trip[], now = new Date()): TripGroup[] {
    const byKey = new Map<string, Trip[]>();
    trips.forEach((trip, idx) => {
      const key = trip.raw.tripKey ?? `${idx}`;
      const members = byKey.get(key) ?? [];
      members.push(trip);
      byKey.set(key, members);
    });
    return [...byKey.values()].map((members) => {
      const sorted = [...members].sort(
        (a, b) => a.departureTime.getTime() - b.departureTime.getTime(),
      );
      const next = sorted.findIndex((trip) => trip.departureTime >= now);
      const start =
        next == -1 ? Math.max(0, sorted.length - this.maxDepartures) : next;
      return new TripGroup(sorted.slice(start, start + this.maxDepartures));
    });
  }

  get first(): Trip {
    return this.trips[0]!;
  }

  includes(trip: Trip): boolean {
    return this.trips.includes(trip);
  }

  /// The first departure yet to leave at `now`, or once they all have, the last.
  nextDeparture(now: Date): Trip {
    return (
      this.trips.find((trip) => trip.departureTime >= now) ??
      this.trips[this.trips.length - 1]!
    );
  }

  /// One per leg. A ride lists every route the group's departures take between its stops, like
  /// Seattle's 1 and 2 Lines; walking or cycling says how many minutes it takes.
  get legBadges(): LegBadge[] {
    return this.first.legs.map((leg, legIdx) => {
      if (!leg.transitLeg) {
        return {
          kind: 'selfPowered',
          emoji: leg.emoji,
          minutes: leg.durationMinutes,
        };
      }
      const routes = new Map<string, RouteBadge>();
      for (const trip of this.trips) {
        const badge = trip.legs[legIdx]!.routeBadge;
        routes.set(badge.label, badge);
      }
      // In the order a rider would count them, not the order they happen to depart in, so the
      // badge doesn't reshuffle as departures roll over.
      const sorted = [...routes.values()].sort((a, b) =>
        a.label.localeCompare(b.label, undefined, { numeric: true }),
      );
      return { kind: 'ride', emoji: leg.emoji, routes: sorted };
    });
  }
}

export type LegBadge =
  | { kind: 'ride'; emoji: string; routes: RouteBadge[] }
  | { kind: 'selfPowered'; emoji: string; minutes: number };
