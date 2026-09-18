import { LngLat } from 'maplibre-gl';
import { i18n } from 'src/i18n/lang';
import { transitVehicleEmoji } from 'src/models/Trip';
import {
  TransitVehicleMode,
  TravelmuxVehicle,
} from 'src/services/TravelmuxClient';
import { formatDuration } from 'src/utils/format';

/// The color of a vehicle whose route doesn't name one, matching an active trip line.
const DEFAULT_VEHICLE_COLOR = '#1296FF';

/// A transit vehicle's last known position, and how to draw it.
export default class TransitVehicle {
  readonly raw: TravelmuxVehicle;
  /// The track's start as epoch millis, parsed once rather than per animation frame.
  private readonly trackStart: number;

  constructor(raw: TravelmuxVehicle) {
    this.raw = raw;
    // The track begins where the vehicle last reported, so that's its start.
    this.trackStart = raw.lastUpdated ? Date.parse(raw.lastUpdated) : NaN;
  }

  /// The color of the chip's ring and the badge's border.
  get color(): string {
    const color = this.raw.route?.color;
    return color ? `#${color}` : DEFAULT_VEHICLE_COLOR;
  }

  /// A vehicle whose feed names no mode is still some kind of transit.
  get emoji(): string {
    return transitVehicleEmoji(
      this.raw.vehicleMode ?? TransitVehicleMode.Transit,
    );
  }

  /// Its route's short name, or its long name when it has none.
  get routeName(): string {
    const route = this.raw.route;
    return route?.shortName ?? route?.longName ?? '';
  }

  /// Only the short name is badged: a long one ("Downtown - Ballard") doesn't fit beside a 22px
  /// dot, so a route without one goes unbadged.
  get badge(): string | undefined {
    return this.raw.route?.shortName;
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
