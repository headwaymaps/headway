import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { i18n } from 'src/i18n/lang';
import {
  OTPClient,
  OTPItinerary,
  OTPItineraryLeg,
  OTPMode,
} from 'src/services/OTPClient';
import { DistanceUnits } from 'src/utils/models';
import {
  formatDistance,
  formatDuration,
  kilometersToMiles,
} from 'src/utils/format';
import { decodeOtpPath } from 'src/third_party/decodePath';

export default class Itinerary {
  private raw: OTPItinerary;
  legs: ItineraryLeg[];
  private distanceUnits: DistanceUnits;

  constructor(otp: OTPItinerary, distanceUnits: DistanceUnits) {
    this.raw = otp;
    this.legs = otp.legs.map((otpLeg) => new ItineraryLeg(otpLeg));
    this.distanceUnits = distanceUnits;
  }

  public static async fetchBest(
    from: LngLat,
    to: LngLat,
    distanceUnits: DistanceUnits
  ): Promise<Itinerary[]> {
    const otpItineraries = await OTPClient.fetchItineraries(from, to, 5);
    return otpItineraries.map((otp) => Itinerary.fromOtp(otp, distanceUnits));
  }

  static fromOtp(raw: OTPItinerary, distanceUnits: DistanceUnits): Itinerary {
    return new Itinerary(raw, distanceUnits);
  }

  public get duration(): number {
    return this.raw.duration;
  }

  public durationFormatted(): string {
    return formatDuration(this.raw.duration, 'shortform');
  }

  public get startTime(): number {
    return this.raw.startTime;
  }

  public startStopTimesFormatted(): string {
    return i18n.global.t('time_range$startTime$endTime', {
      startTime: formatTime(this.startTime),
      endTime: formatTime(this.endTime),
    });
  }

  public get endTime(): number {
    return this.raw.endTime;
  }

  get walkingDistanceMeters(): number {
    return this.raw.walkDistance;
  }

  public walkingDistanceFormatted(): string {
    let distance = this.walkingDistanceMeters;
    if (this.distanceUnits != DistanceUnits.Kilometers) {
      distance = kilometersToMiles(distance / 1000);
    }

    return formatDistance(distance, this.distanceUnits);
  }

  public get viaRouteFormatted(): string | undefined {
    return this.legs.map((leg) => leg.shortName).join('→');
  }

  public get bounds(): LngLatBounds {
    const bounds = new LngLatBounds();
    for (const leg of this.legs) {
      const lineString = leg.geometry();
      for (const coord of lineString.coordinates) {
        bounds.extend([coord[0], coord[1]]);
      }
    }
    return bounds;
  }
}

function formatTime(millis: number): string {
  return new Date(millis).toLocaleTimeString([], { timeStyle: 'short' });
}

class ItineraryLeg {
  readonly raw: OTPItineraryLeg;
  constructor(otp: OTPItineraryLeg) {
    this.raw = otp;
  }

  get shortName(): string {
    switch (this.mode) {
      case OTPMode.Walk:
        return '🚶‍♀️';
      case OTPMode.Bus:
        return '🚍' + this.raw.routeShortName;
      case OTPMode.Tram:
        return '🚊' + this.raw.routeShortName;
      case OTPMode.Train:
        return '🚆' + this.raw.routeShortName;
    }
  }

  get mode(): OTPMode {
    return this.raw.mode;
  }

  geometry(): GeoJSON.LineString {
    const points: [number, number][] = decodeOtpPath(
      this.raw.legGeometry.points
    );
    return {
      type: 'LineString',
      coordinates: points,
    };
  }

  paintStyle(active: boolean): LineLayerSpecification['paint'] {
    return {
      'line-color': active ? (this.transitLeg ? '#E21919' : '#1976D2') : '#777',
      'line-width': this.transitLeg ? 6 : 4,
      'line-dasharray': this.transitLeg ? [1] : [1, 2],
    };
  }

  get transitLeg(): boolean {
    return this.raw.transitLeg;
  }

  get startTime(): number {
    return this.raw.startTime;
  }

  get endTime(): number {
    return this.raw.endTime;
  }
}
