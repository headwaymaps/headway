import { LngLat } from 'maplibre-gl';
import type { BaseMapInterface } from 'src/components/BaseMap.vue';
import { i18n } from 'src/i18n/lang';
import Trip, { transitVehicleEmoji } from 'src/models/Trip';
import {
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

  constructor(raw: TravelmuxVehicle, style: PatternStyle) {
    this.raw = raw;
    this.style = style;
  }

  get lngLat(): LngLat {
    return new LngLat(this.raw.lon, this.raw.lat);
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

  /// How stale this position is, phrased for the traveler - "Location as of 40 sec ago".
  asOfFormatted(now: Date = new Date()): string {
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
    return i18n.global.t('transit_vehicle_location_as_of_$timeDuration', {
      timeDuration,
    });
  }
}

/// Polls for the vehicles serving the transit legs of the trips on screen, and draws each as a
/// pulsing dot until [stop]ped.
///
/// Only some feeds publish positions, so most trips draw nothing at all.
export default class VehicleOverlay {
  private map: BaseMapInterface;
  private from: LngLat;
  private to: LngLat;
  private trips: Trip[];
  private timer?: ReturnType<typeof setInterval>;
  private markerKeys: string[] = [];

  constructor(map: BaseMapInterface, from: LngLat, to: LngLat, trips: Trip[]) {
    this.map = map;
    this.from = from;
    this.to = to;
    this.trips = trips;
  }

  start(): void {
    this.stop();
    // Before the first refresh, so that an in-flight poll can tell it's still wanted.
    this.timer = setInterval(() => this.refresh(), POLL_INTERVAL_MS);
    this.refresh();
  }

  stop(): void {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
    this.clearMarkers();
  }

  /// How to draw each pattern's vehicles, keyed by pattern code.
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
        });
      }
    }
    return styles;
  }

  private clearMarkers(): void {
    for (const key of this.markerKeys) {
      this.map.removeMarker(key);
    }
    this.markerKeys = [];
  }

  private async refresh(): Promise<void> {
    const styles = this.stylesByPattern();
    const result = await TravelmuxClient.fetchVehiclePositions(
      this.from,
      this.to,
      [...styles.keys()],
    );

    if (!result.ok) {
      console.warn('failed to fetch vehicle positions', result.error);
      return;
    }

    // A poll that lands after the page has moved on has nothing left to draw onto.
    if (!this.timer) {
      return;
    }

    this.clearMarkers();
    for (const raw of result.value) {
      const vehicle = new TransitVehicle(
        raw,
        styles.get(raw.patternCode) ?? UNKNOWN_PATTERN,
      );
      const marker = Markers.transitVehicle({
        ...vehicle.style,
        vehicleLabel: vehicle.labelFormatted,
        ageText: () => vehicle.asOfFormatted(),
      }).setLngLat(vehicle.lngLat);
      this.map.pushMarker(vehicle.markerKey, marker);
      this.markerKeys.push(vehicle.markerKey);
    }
  }
}
