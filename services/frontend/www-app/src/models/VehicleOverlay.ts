import { LngLat, Marker } from 'maplibre-gl';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import { i18n } from 'src/i18n/lang';
import Trip, { transitVehicleEmoji } from 'src/models/Trip';
import {
  PatternRequest,
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

/// How a pattern's vehicles are drawn, taken from the leg that put the pattern on screen.
export interface PatternStyle {
  color: string;
  emoji: string;
  /// What to call the route on hover: its short name, or its long name when it has none.
  routeName: string;
  /// Badged onto the marker itself. Only ever the short name - a long one ("Downtown -
  /// Ballard") doesn't fit beside a 22px dot - so a route without one goes unbadged.
  badge?: string;
  /// Where the rider boards this leg. Travelmux reports the vehicles either side of it rather
  /// than every vehicle running the route.
  boardingStop?: LngLat;
}

const UNKNOWN_PATTERN: PatternStyle = {
  color: DEFAULT_VEHICLE_COLOR,
  emoji: '🚍',
  routeName: '',
};

/// A transit vehicle's last known position, and the route color to draw it in.
export class TransitVehicle {
  readonly raw: TravelmuxVehicle;
  readonly style: PatternStyle;
  /// `raw.track`'s timestamps as epoch millis, parsed once rather than per animation frame.
  private readonly trackTimes: number[];

  constructor(raw: TravelmuxVehicle, style: PatternStyle) {
    this.raw = raw;
    this.style = style;
    this.trackTimes = (raw.track ?? []).map((waypoint) =>
      Date.parse(waypoint.time),
    );
  }

  /// Where the vehicle last actually reported being.
  get lngLat(): LngLat {
    return new LngLat(this.raw.lon, this.raw.lat);
  }

  /// Where we reckon the vehicle is at `now`, walking the predicted track.
  ///
  /// Before the track begins, or with no track at all, that's just the reported position. Past
  /// its end we hold at the last point rather than carrying on off the end of the prediction.
  positionAt(now: number): LngLat {
    const track = this.raw.track;
    if (!track || track.length === 0) {
      return this.lngLat;
    }
    if (now <= this.trackTimes[0]!) {
      return new LngLat(track[0]!.lon, track[0]!.lat);
    }
    const end = track.length - 1;
    if (now >= this.trackTimes[end]!) {
      return new LngLat(track[end]!.lon, track[end]!.lat);
    }
    for (let i = 1; i <= end; i++) {
      const until = this.trackTimes[i]!;
      if (until < now) {
        continue;
      }
      const since = this.trackTimes[i - 1]!;
      const from = track[i - 1]!;
      const to = track[i]!;
      const span = until - since;
      const into = span > 0 ? (now - since) / span : 0;
      return new LngLat(
        from.lon + into * (to.lon - from.lon),
        from.lat + into * (to.lat - from.lat),
      );
    }
    return this.lngLat;
  }

  /// Whether the dot has moved past the last thing the vehicle actually told us.
  isEstimatedAt(now: number): boolean {
    return this.trackTimes.length > 0 && now > this.trackTimes[0]!;
  }

  /// Stable for as long as the vehicle keeps reporting, so it can key a marker.
  get markerKey(): string {
    return `vehicle_${this.raw.patternCode}_${this.raw.vehicleId ?? this.raw.label ?? ''}`;
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

  /// The route color to draw each pattern's vehicles in, keyed by pattern code.
  private stylesByPattern(): Map<string, PatternStyle> {
    const styles = new Map<string, PatternStyle>();
    for (const trip of this.trips) {
      for (const leg of trip.legs) {
        const transitLeg = leg.raw.transitLeg;
        if (!transitLeg?.patternCode) {
          continue;
        }
        const route = transitLeg.route;
        styles.set(transitLeg.patternCode, {
          color: route?.color ? `#${route.color}` : DEFAULT_VEHICLE_COLOR,
          emoji: transitVehicleEmoji(transitLeg.vehicleMode),
          routeName: route?.shortName ?? route?.longName ?? '',
          badge: route?.shortName,
          boardingStop: leg.sourceLngLat,
        });
      }
    }
    return styles;
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
      const now = Date.now();
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
    const styles = this.stylesByPattern();
    const patterns: PatternRequest[] = [...styles].map(([code, style]) => ({
      code,
      boardingStop: style.boardingStop,
    }));
    const result = await TravelmuxClient.fetchVehiclePositions(
      this.from,
      this.to,
      patterns,
    );

    if (!result.ok) {
      console.warn('failed to fetch vehicle positions', result.error);
      return;
    }

    // A poll that lands after the page has moved on has nothing left to draw onto.
    if (!this.timer) {
      return;
    }

    const stale = new Set(this.tracked.keys());
    for (const raw of result.value) {
      const vehicle = new TransitVehicle(
        raw,
        styles.get(raw.patternCode) ?? UNKNOWN_PATTERN,
      );
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
          this.tracked.get(key)?.vehicle.freshnessFormatted() ?? '',
      }).setLngLat(vehicle.positionAt(Date.now()));
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
