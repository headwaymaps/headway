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

/// Where a vehicle is expected to be over the next few minutes, with its points ready to
/// interpolate between.
interface Track {
  stepSeconds: number;
  points: LngLat[];
}

/// A transit vehicle's last known position, and how to draw it.
export default class TransitVehicle {
  readonly raw: TravelmuxVehicle;
  /// Where the vehicle last actually reported being.
  readonly lngLat: LngLat;
  /// When it reported, as epoch millis - which is also where its track starts.
  private readonly reportedAt: number;
  /// The track, read off the wire once rather than per animation frame.
  private readonly track?: Track;

  constructor(raw: TravelmuxVehicle) {
    this.raw = raw;
    this.lngLat = new LngLat(...raw.position);
    this.reportedAt = Date.parse(raw.lastUpdated);
    this.track = raw.track && {
      stepSeconds: raw.track.stepSeconds,
      points: raw.track.points.map((point) => new LngLat(...point)),
    };
  }

  /// The color of the chip's ring and the badge's border.
  get color(): string {
    const color = this.raw.route.color;
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
    return route.shortName ?? route.longName ?? '';
  }

  /// Only the short name is badged: a long one ("Downtown - Ballard") doesn't fit beside a 22px
  /// dot, so a route without one goes unbadged.
  get badge(): string | undefined {
    return this.raw.route.shortName;
  }

  /// Where we reckon the vehicle is at `now`, walking the predicted track.
  ///
  /// The track's points are evenly spaced in time, so the pair either side of `now` is an index,
  /// not a search. Before it begins, or with no track at all, that's just the reported position;
  /// past its end we hold at the last point rather than running off the end of the prediction.
  positionAt(now: number): LngLat {
    const track = this.track;
    if (!track) {
      return this.lngLat;
    }

    const elapsed = (now - this.reportedAt) / 1000 / track.stepSeconds;
    if (elapsed <= 0) {
      return track.points[0]!;
    }
    const last = track.points.length - 1;
    if (elapsed >= last) {
      return track.points[last]!;
    }

    const i = Math.floor(elapsed);
    const into = elapsed - i;
    const from = track.points[i]!;
    const to = track.points[i + 1]!;
    return new LngLat(
      from.lng + into * (to.lng - from.lng),
      from.lat + into * (to.lat - from.lat),
    );
  }

  /// Whether the dot has moved past the last thing the vehicle actually told us.
  isEstimatedAt(now: number): boolean {
    return !!this.track && now > this.reportedAt;
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
    const ageSeconds = Math.max(0, (now.getTime() - this.reportedAt) / 1000);
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
