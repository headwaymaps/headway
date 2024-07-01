import { LngLat } from 'maplibre-gl';
import { i18n } from 'src/i18n/lang';
import {
  OTPAlert,
  OTPError,
  OTPErrorId,
  OTPItinerary,
  OTPItineraryLeg,
  OTPMode,
} from 'src/services/OpenTripPlannerAPI';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { formatDistance, formatTime } from 'src/utils/format';

export enum ItineraryErrorCode {
  Other,
  SourceOutsideBounds,
  DestinationOutsideBounds,
  TransitServiceDisabled,
}

export class ItineraryError {
  errorCode: ItineraryErrorCode;
  message?: string;

  constructor(errorType: ItineraryErrorCode, message: string) {
    this.errorCode = errorType;
    this.message = message;
  }

  static fromOtp(otpError: OTPError): ItineraryError {
    if ('planError' in otpError) {
      const planError = otpError.planError;
      switch (planError.id) {
        case OTPErrorId.OutsideBounds: {
          if (planError.missing.includes('TO_PLACE')) {
            return {
              errorCode: ItineraryErrorCode.DestinationOutsideBounds,
              message: planError.msg,
            };
          } else {
            console.assert(planError.missing.includes('FROM_PLACE'));
            return {
              errorCode: ItineraryErrorCode.SourceOutsideBounds,
              message: planError.msg,
            };
          }
        }
        default: {
          return {
            errorCode: ItineraryErrorCode.Other,
            message: planError.message,
          };
        }
      }
    } else {
      const responseError = otpError.responseError;
      switch (responseError.status) {
        case 404: {
          return {
            errorCode: ItineraryErrorCode.TransitServiceDisabled,
          };
        }
        default: {
          return {
            errorCode: ItineraryErrorCode.Other,
          };
        }
      }
    }
  }
}

export default class Itinerary {
  private raw: OTPItinerary;
  legs: ItineraryLeg[];
  private preferredDistanceUnits: DistanceUnits;
  withBicycle: boolean;

  constructor(
    otp: OTPItinerary,
    distanceUnits: DistanceUnits,
    withBicycle: boolean,
  ) {
    this.raw = otp;
    this.legs = otp.legs.map(
      (otpLeg: OTPItineraryLeg) => new ItineraryLeg(otpLeg),
    );
    this.preferredDistanceUnits = distanceUnits;
    this.withBicycle = withBicycle;
  }
  mode: TravelMode = TravelMode.Transit;

  static fromOtp(
    raw: OTPItinerary,
    preferredDistanceUnits: DistanceUnits,
    withBicycle: boolean,
  ): Itinerary {
    return new Itinerary(raw, preferredDistanceUnits, withBicycle);
  }

  public get duration(): number {
    return this.raw.duration;
  }

  public get startTime(): number {
    return this.raw.startTime;
  }

  public get startStopTimesFormatted(): string {
    return i18n.global.t('time_range$startTime$endTime', {
      startTime: formatTime(this.startTime),
      endTime: formatTime(this.endTime),
    });
  }

  public get endTime(): number {
    return this.raw.endTime;
  }

  // Usually walking, but will be biking if mode is transit+bike
  get walkDistance(): number {
    return this.raw.walkDistance;
  }

  public get walkingDistanceFormatted(): string {
    const preformattedDistance = formatDistance(
      this.walkDistance,
      DistanceUnits.Meters, // OTP is always metric
      this.preferredDistanceUnits,
    );

    if (this.withBicycle) {
      return i18n.global.t('bike_distance', { preformattedDistance });
    } else {
      return i18n.global.t('walk_distance', { preformattedDistance });
    }
  }

  public get alerts(): LegAlert[] {
    return this.legs.map((l) => l.alerts).flat();
  }

  public get firstTransitLeg(): ItineraryLeg | undefined {
    return this.legs.slice(0, 2).find((leg) => leg.transitLeg);
  }

  public get hasAlerts(): boolean {
    for (const leg of this.legs) {
      if (leg.alerts.length > 0) {
        return true;
      }
    }
    return false;
  }
}

export class ItineraryLeg {
  readonly raw: OTPItineraryLeg;
  constructor(otp: OTPItineraryLeg) {
    this.raw = otp;
  }

  get emoji(): string {
    switch (this.raw.mode) {
      case OTPMode.Walk:
        return '🚶‍♀️';
      case OTPMode.Bus:
      case OTPMode.Transit:
        return '🚍';
      case OTPMode.Rail:
        return '🚆';
      case OTPMode.Subway:
        return '🚇';
      case OTPMode.Bicycle:
        return '🚲';
      case OTPMode.CableCar:
      case OTPMode.Tram:
        return '🚊';
      case OTPMode.Funicular:
        return '🚡';
      case OTPMode.Gondola:
        return '🚠';
      case OTPMode.Car:
        return '🚙';
      case OTPMode.Ferry:
        return '⛴️';
      default:
        console.error('no emoji for mode', this.mode);
        return '';
    }
  }

  get shortName(): string {
    const emoji = this.emoji;
    const shortName = this.raw.routeShortName ?? this.raw.route;
    return `${emoji} ${shortName}`.trim();
  }

  get mode(): TravelMode {
    return travelModeFromOtpMode(this.raw.mode);
  }

  get sourceName(): string {
    return this.raw.from.name;
  }

  get destinationName(): string {
    return this.raw.to.name;
  }

  get sourceLngLat(): LngLat {
    return new LngLat(this.raw.from.lon, this.raw.from.lat);
  }

  get destinationLngLat(): LngLat {
    return new LngLat(this.raw.to.lon, this.raw.to.lat);
  }

  get duration(): number {
    return (this.raw.endTime - this.raw.startTime) / 1000;
  }

  get transitLeg(): boolean {
    return this.raw.transitLeg;
  }

  get startTime(): number {
    console.assert(this.raw.startTime !== undefined, 'start time is undefined');
    return this.raw.startTime;
  }

  get endTime(): number {
    console.assert(this.raw.endTime !== undefined, 'end time is undefined');
    return this.raw.endTime;
  }

  get realTime(): boolean {
    return this.raw.realTime;
  }

  get departureLocationName(): string | undefined {
    return this.raw.from?.name;
  }

  get alerts(): LegAlert[] {
    return this.raw.alerts?.map((a) => new LegAlert(a)) || [];
  }
}

class LegAlert {
  raw: OTPAlert;

  constructor(otp: OTPAlert) {
    this.raw = otp;
  }

  get headerText(): string {
    return this.raw.alertHeaderText;
  }

  get descriptionText(): string {
    return this.raw.alertDescriptionText;
  }
}

function travelModeFromOtpMode(mode: OTPMode): TravelMode {
  switch (mode) {
    case OTPMode.Walk:
      return TravelMode.Walk;
    case OTPMode.Bicycle:
      return TravelMode.Bike;
    case OTPMode.Transit:
    case OTPMode.Bus:
      return TravelMode.Transit;
    case OTPMode.Car:
      return TravelMode.Drive;
    default:
      console.assert(false, `assuming transit for unhandled otp mode: ${mode}`);
      return TravelMode.Transit;
  }
}
