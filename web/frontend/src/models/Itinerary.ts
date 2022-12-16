import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { i18n } from 'src/i18n/lang';
import {
  OTPClient,
  OTPError,
  OTPErrorId,
  OTPItinerary,
  OTPItineraryLeg,
  OTPMode,
} from 'src/services/OTPClient';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import {
  formatDistance,
  formatDuration,
  formatTime,
  kilometersToMiles,
} from 'src/utils/format';
import { decodeOtpPath } from 'src/third_party/decodePath';
import Trip from './Trip';
import { Err, Ok, Result } from 'src/utils/Result';

export enum ItineraryErrorCode {
  Other,
  SourceOutsideBounds,
  DestinationOutsideBounds,
}

export class ItineraryError {
  errorCode: ItineraryErrorCode;
  message: string;

  constructor(errorType: ItineraryErrorCode, message: string) {
    this.errorCode = errorType;
    this.message = message;
  }

  static fromOtp(otpError: OTPError): ItineraryError {
    switch (otpError.id) {
      case OTPErrorId.OutsideBounds: {
        if (otpError.missing.includes('TO_PLACE')) {
          return {
            errorCode: ItineraryErrorCode.DestinationOutsideBounds,
            message: otpError.msg,
          };
        } else {
          console.assert(otpError.missing.includes('FROM_PLACE'));
          return {
            errorCode: ItineraryErrorCode.SourceOutsideBounds,
            message: otpError.msg,
          };
        }
      }
      default: {
        return {
          errorCode: ItineraryErrorCode.Other,
          message: otpError.message,
        };
      }
    }
  }
}

export default class Itinerary implements Trip {
  private raw: OTPItinerary;
  legs: ItineraryLeg[];
  private distanceUnits: DistanceUnits;

  constructor(otp: OTPItinerary, distanceUnits: DistanceUnits) {
    this.raw = otp;
    this.legs = otp.legs.map((otpLeg) => new ItineraryLeg(otpLeg));
    this.distanceUnits = distanceUnits;
  }
  // We leave this blank for transit itineraries. It's not really relevant to
  // picking a trip, so we don't clutter the screen with it.
  lengthFormatted?: string | undefined;
  mode: TravelMode = TravelMode.Transit;

  public static async fetchBest(
    from: LngLat,
    to: LngLat,
    distanceUnits: DistanceUnits
  ): Promise<Result<Itinerary[], ItineraryError>> {
    const result = await OTPClient.fetchItineraries(from, to, 5);
    if (result.ok) {
      return Ok(
        result.value.map((otp) => Itinerary.fromOtp(otp, distanceUnits))
      );
    } else {
      return Err(ItineraryError.fromOtp(result.error));
    }
  }

  static fromOtp(raw: OTPItinerary, distanceUnits: DistanceUnits): Itinerary {
    return new Itinerary(raw, distanceUnits);
  }

  public get duration(): number {
    return this.raw.duration;
  }

  public get durationFormatted(): string {
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

export class ItineraryLeg {
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
    return this.raw.startTime;
  }

  get endTime(): number {
    return this.raw.endTime;
  }
}
