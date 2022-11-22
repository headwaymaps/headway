import { LngLat } from 'maplibre-gl';
import { i18n } from 'src/i18n/lang';
import {
  OTPClient,
  OTPItinerary,
  OTPItineraryLeg,
  OTPMode,
  OTPLegGeometry,
} from 'src/services/OTPClient';
import { DistanceUnits } from 'src/utils/models';
import {
  formatDistance,
  formatDuration,
  kilometersToMiles,
} from 'src/utils/format';

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

  get legGeometry(): OTPLegGeometry {
    return this.raw.legGeometry;
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
