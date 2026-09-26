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

/// Within this much of the boarding stop, a countdown is less use to a waiting rider than being
/// told to look up.
const ARRIVING_NOW_SECONDS = 30;

/// How far past its predicted arrival a vehicle is still called arriving.
const PAST_STOP_GRACE_SECONDS = 15;

/// How long a vehicle keeps being drawn on its last report alone, matching how far ahead
/// travelmux predicts.
const COASTING_MS = 3 * 60 * 1000;

/// A duration under a minute, which `formatDuration` would round up to "1 min" and overstate.
function shortDuration(seconds: number): string {
  return seconds < 60
    ? i18n.global.t('times_shortform.$n_seconds', { n: Math.round(seconds) })
    : formatDuration(seconds, 'shortform');
}

/// A countdown split at the unit, so a view can set the unit in smaller type than the number.
function countdown(seconds: number): Countdown {
  if (seconds < 60) {
    return {
      value: `${Math.round(seconds)}`,
      unit: i18n.global.t('times_unit.seconds'),
    };
  }
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) {
    return {
      value: `${minutes}`,
      unit: i18n.global.t('times_unit.minutes'),
    };
  }
  const hours = Math.floor(minutes / 60);
  return {
    value: `${hours}:${`${minutes - hours * 60}`.padStart(2, '0')}`,
    unit: i18n.global.t('times_unit.hours'),
  };
}

/// How long until a vehicle reaches the stop, with the unit kept apart from the number.
export type Countdown = { value: string; unit?: string };

/// What the popover says about the rider's boarding stop: where the vehicle is in the stop
/// sequence, and - while it's still on its way - how long the wait is.
export type BoardingStopRow = {
  /// Absent when the feed won't say how many stops are left - the countdown says the rest.
  text?: string;
  countdown?: Countdown;
};

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
  /// When the vehicle is at each stop up to and including the rider's, as epoch millis.
  private readonly stopArrivals: number[];

  constructor(raw: TravelmuxVehicle) {
    this.raw = raw;
    this.lngLat = new LngLat(...raw.position);
    this.reportedAt = Date.parse(raw.lastUpdated);
    this.track = raw.track && {
      stepSeconds: raw.track.stepSeconds,
      points: raw.track.points.map((point) => new LngLat(...point)),
    };
    this.stopArrivals =
      raw.boardingStop?.state === 'approaching'
        ? (raw.boardingStop.stopArrivals ?? []).map(Date.parse)
        : [];
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

  /// Whether the report this dot is drawn from is too old to keep drawing without a fresh one.
  hasExpiredAt(now: number): boolean {
    return now - this.reportedAt > COASTING_MS;
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

  /// The boarding-stop line of the popover: how far up the route the vehicle still is, and how
  /// long until it gets here.
  ///
  /// Undefined when travelmux had nothing to say about the stop - the marker then just carries
  /// the route and how fresh the position is.
  boardingStopRow(now: Date = new Date()): BoardingStopRow | undefined {
    const boardingStop = this.raw.boardingStop;
    if (!boardingStop) {
      return undefined;
    }

    const seconds = (Date.parse(boardingStop.arrival) - now.getTime()) / 1000;
    if (boardingStop.state === 'departed') {
      // Nothing left to wait through, so no countdown - just how long ago it went by.
      return {
        text: i18n.global.t('transit_vehicle_departed_$timeDuration', {
          timeDuration: shortDuration(Math.max(0, -seconds)),
        }),
      };
    }

    // Only once it's clearly gone: these positions are predictions, and a rider still at the stop
    // shouldn't be told they've been passed on the strength of a few seconds' drift.
    if (seconds < -PAST_STOP_GRACE_SECONDS) {
      return { text: i18n.global.t('transit_vehicle_past_stop') };
    }
    if (seconds <= ARRIVING_NOW_SECONDS) {
      // Which stop it's at matters less than telling a waiting rider to look up.
      return { text: i18n.global.t('transit_vehicle_arriving_now') };
    }
    return {
      text: this.stopsAwayFormatted(now),
      countdown: countdown(seconds),
    };
  }

  /// How many stops until the vehicle reaches the rider's at `now`, counting that stop itself.
  ///
  /// Counted off the same predictions that place the dot, so the number comes down as the dot
  /// passes stops rather than holding at whatever the last poll said. Undefined for a vehicle
  /// that has been past: a count of stops is something to wait through, and one that's gone by
  /// is nothing to wait for.
  stopsAwayFormatted(now: Date = new Date()): string | undefined {
    const stopsAway = this.stopArrivals.filter(
      (arrival) => arrival > now.getTime(),
    ).length;
    if (stopsAway === 0) {
      return undefined;
    }
    return stopsAway === 1
      ? i18n.global.t('transit_vehicle_next_stop')
      : i18n.global.t('transit_vehicle_$n_stops_away', { n: stopsAway });
  }

  /// How much of this dot is reported and how much is guesswork, phrased for the traveler.
  ///
  /// Once the dot has left the reported position it says so: the position on screen is one
  /// nobody reported, and the honest thing is to name the last moment we actually knew.
  freshnessFormatted(now: Date = new Date()): string {
    const ageSeconds = Math.max(0, (now.getTime() - this.reportedAt) / 1000);
    const timeDuration = shortDuration(ageSeconds);
    const phrase = this.isEstimatedAt(now.getTime())
      ? 'transit_vehicle_location_estimated_$timeDuration'
      : 'transit_vehicle_location_as_of_$timeDuration';
    return i18n.global.t(phrase, { timeDuration });
  }
}
