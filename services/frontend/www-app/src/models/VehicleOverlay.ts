import { LngLat, Marker } from 'maplibre-gl';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import { i18n } from 'src/i18n/lang';
import Trip, { transitVehicleEmoji } from 'src/models/Trip';
import {
  PatternRequest,
  TransitVehicleMode,
  TravelmuxClient,
  TravelmuxVehicle,
} from 'src/services/TravelmuxClient';
import { formatDuration } from 'src/utils/format';
import Markers from 'src/utils/Markers';

/// OTP polls its GTFS-RT vehicle position updaters once a minute, so asking much more often than
/// this just re-fetches a position we already have.
const POLL_INTERVAL_MS = 30_000;

/// The color of a vehicle whose route doesn't name one, matching an active trip line.
const DEFAULT_VEHICLE_COLOR = '#1296FF';

/// A transit vehicle's last known position, and how to draw it.
export class TransitVehicle {
  readonly raw: TravelmuxVehicle;
  /// The track's start as epoch millis, parsed once rather than per animation frame.
  private readonly trackStart: number;

  constructor(raw: TravelmuxVehicle) {
    this.raw = raw;
    // The track begins where the vehicle last reported, so that's its start.
    this.trackStart = raw.lastUpdated ? Date.parse(raw.lastUpdated) : NaN;
  }

  /// How to draw this vehicle. Everything here rides along on the vehicle itself, so there's no
  /// looking back at the leg that put its pattern on screen.
  get style(): {
    color: string;
    emoji: string;
    routeName: string;
    badge?: string;
  } {
    const route = this.raw.route;
    return {
      color: route?.color ? `#${route.color}` : DEFAULT_VEHICLE_COLOR,
      // A vehicle whose feed names no mode is still some kind of transit.
      emoji: transitVehicleEmoji(
        this.raw.vehicleMode ?? TransitVehicleMode.Transit,
      ),
      /// Its short name, or its long name when it has none.
      routeName: route?.shortName ?? route?.longName ?? '',
      /// Only the short name is badged: a long one ("Downtown - Ballard") doesn't fit beside a
      /// 22px dot, so a route without one goes unbadged.
      badge: route?.shortName,
    };
  }

  /// Where the vehicle last actually reported being.
  get lngLat(): LngLat {
    return new LngLat(this.raw.lon, this.raw.lat);
  }

  /// Where we reckon the vehicle is at `now`, walking the predicted track.
  ///
  /// The track's points are evenly spaced in time, so the pair either side of `now` is an index,
  /// not a search. Before it begins, or with no track at all, that's just the reported position;
  /// past its end we hold at the last point rather than running off the end of the prediction.
  positionAt(now: number): LngLat {
    const track = this.raw.track;
    if (!track || track.points.length === 0) {
      return this.lngLat;
    }
    const at = (i: number) =>
      new LngLat(track.points[i]![1], track.points[i]![0]);

    const elapsed = (now - this.trackStart) / 1000 / track.stepSeconds;
    if (!(elapsed > 0)) {
      return at(0);
    }
    const last = track.points.length - 1;
    if (elapsed >= last) {
      return at(last);
    }

    const i = Math.floor(elapsed);
    const into = elapsed - i;
    const from = track.points[i]!;
    const to = track.points[i + 1]!;
    return new LngLat(
      from[1] + into * (to[1] - from[1]),
      from[0] + into * (to[0] - from[0]),
    );
  }

  /// Whether the dot has moved past the last thing the vehicle actually told us.
  isEstimatedAt(now: number): boolean {
    return !!this.raw.track && now > this.trackStart;
  }

  /// Stable for as long as the vehicle keeps reporting, so it can key a marker.
  get markerKey(): string {
    return `vehicle_${this.raw.id}`;
  }

  /// The number painted on the vehicle, phrased for the tooltip - "(vehicle 7193)".
  ///
  /// Buses publish one; Link, Sounder and the ferries don't, so this is often absent. It comes
  /// from the feed's `label` rather than its `vehicleId`: for King County Metro's DART vans the
  /// two disagree, and `label` is the one that matches the van.
  get labelFormatted(): string | undefined {
    if (!this.raw.label) {
      return undefined;
    }
    return i18n.global.t('transit_vehicle_$label', { label: this.raw.label });
  }

  /// How much of this dot is reported and how much is guesswork, phrased for the traveler.
  ///
  /// Once the dot has left the reported position it says so: the position on screen is one
  /// nobody reported, and the honest thing is to name the last moment we actually knew.
  freshnessFormatted(now: Date = new Date()): string {
    if (!this.raw.lastUpdated) {
      return i18n.global.t('transit_vehicle_location_live');
    }
    const ageSeconds = Math.max(
      0,
      (now.getTime() - new Date(this.raw.lastUpdated).getTime()) / 1000,
    );
    // Positions refresh about once a minute, so most ages land under one - where formatDuration
    // would round 40 seconds up to "1 min" and overstate how fresh this is.
    const timeDuration =
      ageSeconds < 60
        ? i18n.global.t('times_shortform.$n_seconds', {
            n: Math.round(ageSeconds),
          })
        : formatDuration(ageSeconds, 'shortform');
    const phrase = this.isEstimatedAt(now.getTime())
      ? 'transit_vehicle_location_estimated_$timeDuration'
      : 'transit_vehicle_location_as_of_$timeDuration';
    return i18n.global.t(phrase, { timeDuration });
  }
}

/// A vehicle we're drawing, and the marker drawing it. The marker outlives a poll so it can be
/// animated between them - and so a tooltip being read doesn't vanish underneath the reader.
interface TrackedVehicle {
  marker: Marker;
  vehicle: TransitVehicle;
}

/// Polls for the vehicles serving the transit legs of the trips on screen, draws each as a
/// pulsing dot, and walks it along travelmux's predicted track between polls until [stop]ped.
///
/// Only some feeds publish positions, so most trips draw nothing at all.
export default class VehicleOverlay {
  private map: BaseMapInterface;
  private from: LngLat;
  private to: LngLat;
  private trips: Trip[];
  private timer?: ReturnType<typeof setInterval>;
  private animation?: number;
  private tracked: Map<string, TrackedVehicle> = new Map();
  /// Milliseconds between this device's clock and the one that timed the tracks. A device even a
  /// few minutes out would otherwise hold every vehicle at one end of its track.
  private clockOffsetMs = 0;
  /// The patterns of the trip the traveler has picked. Vehicles on any other trip's patterns are
  /// drawn dimmed. Empty means nothing is picked yet, and nothing is dimmed.
  private selected: Set<string> = new Set();

  constructor(map: BaseMapInterface, from: LngLat, to: LngLat, trips: Trip[]) {
    this.map = map;
    this.from = from;
    this.to = to;
    this.trips = trips;
  }

  /// Bring the picked trip's vehicles forward and dim the rest.
  selectTrip(trip?: Trip): void {
    this.selected = new Set(trip?.patternCodes ?? []);
    for (const tracked of this.tracked.values()) {
      Markers.setTransitVehicleFaded(
        tracked.marker,
        this.isFaded(tracked.vehicle.raw.patternCode),
      );
    }
  }

  private isFaded(patternCode: string): boolean {
    return this.selected.size > 0 && !this.selected.has(patternCode);
  }

  start(): void {
    this.stop();
    // Before the first refresh, so that an in-flight poll can tell it's still wanted.
    this.timer = setInterval(() => void this.poll(), POLL_INTERVAL_MS);
    void this.poll();
    this.animate();
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
    if (this.animation !== undefined) {
      cancelAnimationFrame(this.animation);
      this.animation = undefined;
    }
    this.clearMarkers();
  }

  /// The patterns on screen and where the rider boards each: travelmux reports the vehicles
  /// either side of that stop rather than every vehicle running the route.
  private patternsToPoll(): PatternRequest[] {
    const boardingStops = new Map<string, LngLat>();
    for (const trip of this.trips) {
      for (const leg of trip.legs) {
        const patternCode = leg.raw.transitLeg?.patternCode;
        if (patternCode) {
          boardingStops.set(patternCode, leg.sourceLngLat);
        }
      }
    }
    return [...boardingStops].map(([code, boardingStop]) => ({
      code,
      boardingStop,
    }));
  }

  private clearMarkers(): void {
    for (const key of this.tracked.keys()) {
      this.map.removeMarker(key);
    }
    this.tracked.clear();
  }

  /// Walk every dot along its track. Positions are interpolated per frame rather than per poll,
  /// which is the whole point: a vehicle reports about once a minute but moves continuously.
  private animate(): void {
    const frame = () => {
      const now = Date.now() + this.clockOffsetMs;
      for (const tracked of this.tracked.values()) {
        tracked.marker.setLngLat(tracked.vehicle.positionAt(now));
      }
      this.animation = requestAnimationFrame(frame);
    };
    this.animation = requestAnimationFrame(frame);
  }

  /// [refresh], with anything it throws logged rather than left as an unhandled rejection -
  /// nothing awaits it, since it's driven by a timer.
  private async poll(): Promise<void> {
    try {
      await this.refresh();
    } catch (e) {
      console.warn('vehicle position refresh failed', e);
    }
  }

  private async refresh(): Promise<void> {
    const result = await TravelmuxClient.fetchVehiclePositions(
      this.from,
      this.to,
      this.patternsToPoll(),
    );

    if (!result.ok) {
      console.warn('failed to fetch vehicle positions', result.error);
      return;
    }

    // A poll that lands after the page has moved on has nothing left to draw onto.
    if (!this.timer) {
      return;
    }

    this.clockOffsetMs = result.value.clockOffsetMs;

    const stale = new Set(this.tracked.keys());
    for (const raw of result.value.vehicles) {
      const vehicle = new TransitVehicle(raw);
      const key = vehicle.markerKey;
      stale.delete(key);

      const existing = this.tracked.get(key);
      if (existing) {
        // Keep the marker: re-creating it restarts the pulse and drops any open tooltip.
        existing.vehicle = vehicle;
        Markers.setTransitVehicleFaded(
          existing.marker,
          this.isFaded(raw.patternCode),
        );
        continue;
      }

      const marker = Markers.transitVehicle({
        ...vehicle.style,
        vehicleLabel: vehicle.labelFormatted,
        // Reads through the map so it picks up each refresh's vehicle, not the one it was
        // built with.
        ageText: () =>
          this.tracked
            .get(key)
            ?.vehicle.freshnessFormatted(
              new Date(Date.now() + this.clockOffsetMs),
            ) ?? '',
      }).setLngLat(vehicle.positionAt(Date.now() + this.clockOffsetMs));
      Markers.setTransitVehicleFaded(marker, this.isFaded(raw.patternCode));
      this.tracked.set(key, { marker, vehicle });
      this.map.pushMarker(key, marker);
    }

    // Vehicles that stopped reporting, or left the patterns we asked about.
    for (const key of stale) {
      this.map.removeMarker(key);
      this.tracked.delete(key);
    }
  }
}
