import { LngLat, LngLatLike } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Ok, Err, Result } from 'src/utils/Result';
import Trip, { TripFetchError } from 'src/models/Trip';
import {
  OTPPlanResponse,
  OTPItinerary,
  OTPItineraryLeg,
} from './OpenTripPlannerAPI';
import { ValhallaRouteResponse, ValhallaErrorCode } from './ValhallaAPI';
import Itinerary from 'src/models/Itinerary';
import { zipWith } from 'lodash';

export interface TravelmuxPlanResponse {
  _otp: OTPPlanResponse;
  _valhalla: ValhallaRouteResponse;
  plan: TravelmuxPlan;
}

export interface TravelmuxPlan {
  itineraries: TravelmuxItinerary[];
}

export interface TravelmuxLeg {
  mode: TravelmuxMode;
  distanceMeters: number;
  duration: number;
  geometry: string;
  transitLeg?: OTPItineraryLeg;
  nonTransitLeg?: NonTransitLeg;
}

export interface NonTransitLeg {
  maneuvers: [TravelmuxManeuver];
  substantialStreetNames?: string[];
}

export interface TravelmuxManeuver {
  instruction?: string;
  verbalPostTransitionInstruction?: string;
  startPoint: LngLatLike;
  // same as valhalla's maneuver type
  type: number;
}

export interface TravelmuxItinerary {
  mode: TravelmuxMode;
  duration: number;
  distance: number;
  distanceUnits: DistanceUnits;
  bounds: { min: [number, number]; max: [number, number] };
  legs: TravelmuxLeg[];
}

// Non-exaustive
export enum TravelmuxErrorCode {
  TransitUnsupportedArea = 1701,

  // Currently, errors originating in Valhalla are +2000
  ValhallaUnsupportedArea = ValhallaErrorCode.UnsupportedArea + 2000,

  // Errors originating in OpenTripPlanner are +3000
}

export interface TravelmuxError {
  errorCode: TravelmuxErrorCode;
  statusCode: number;
  message: string;
}

// incomplete
export type TravelmuxPlanRequest = {
  fromPlace: string;
  toPlace: string;
  // It'd be nice to typecheck this as numeric, but it would require some
  // additional type juggling elsewhere
  //numItineraries?: number,
  numItineraries?: string;
  time?: string;
  date?: string;
  arriveBy?: string;
  // comma separated list Mode(s)
  mode?: string;
  preferredDistanceUnits: string;
};

export enum TravelmuxMode {
  Bike = 'BICYCLE',
  Walk = 'WALK',
  Drive = 'CAR',
  Transit = 'TRANSIT',
}

export class TravelmuxClient {
  public static async fetchPlans(
    from: LngLat,
    to: LngLat,
    modes: TravelmuxMode[],
    numItineraries: number,
    preferredDistanceUnits: DistanceUnits,
    time?: string,
    date?: string,
    arriveBy?: boolean,
  ): Promise<Result<Trip[], TripFetchError>> {
    const params: TravelmuxPlanRequest = {
      fromPlace: `${from.lat},${from.lng}`,
      toPlace: `${to.lat},${to.lng}`,
      numItineraries: `${numItineraries}`,
      mode: modes.join(','),
      preferredDistanceUnits,
    };

    // The OTP API assumes current date and time if neither are specified.
    // If only date is specified, the current time at that date is assumed.
    // If only time is specified, it's an error.
    if (time) {
      console.assert(
        date,
        'The OTP API requires that if time is specified, date must also be specified',
      );
      params['time'] = time;
    }
    if (date) {
      params['date'] = date;
    }
    if (arriveBy) {
      params['arriveBy'] = true.toString();
    }

    const query = new URLSearchParams(params).toString();

    const response = await fetch('/travelmux/v6/plan?' + query);

    if (response.ok) {
      const travelmuxResponseJson: TravelmuxPlanResponse =
        await response.json();

      const tmxItineraries = travelmuxResponseJson.plan.itineraries;

      if (travelmuxResponseJson._otp) {
        const otpItineraries = travelmuxResponseJson._otp.plan.itineraries;
        const trips = zipWith(
          tmxItineraries,
          otpItineraries,
          (tmxItinerary: TravelmuxItinerary, otpRawItinerary: OTPItinerary) => {
            const otpItinerary = Itinerary.fromOtp(
              otpRawItinerary,
              preferredDistanceUnits,
              modes.includes(TravelmuxMode.Bike),
            );
            // OTP always returns metric units
            return new Trip(
              tmxItinerary,
              preferredDistanceUnits,
              otpItinerary,
              DistanceUnits.Kilometers,
            );
          },
        );
        return Ok(trips);
      } else {
        console.assert(
          travelmuxResponseJson._valhalla,
          'expected valhalla in non-transit response',
        );
        const trips = tmxItineraries.map((tmxItinerary: TravelmuxItinerary) => {
          console.assert(tmxItinerary, 'expected tmxItinerary to be set');
          return new Trip(
            tmxItinerary,
            preferredDistanceUnits,
            null,
            tmxItinerary.distanceUnits,
          );
        });
        return Ok(trips);
      }
    } else {
      const errorBody = await response.json();
      const error = errorBody['error'];
      console.assert(error);
      const routeError = TripFetchError.fromTravelmux(error);
      return Err(routeError);
    }
  }
}

export function travelModeFromTravelmuxMode(mode: TravelmuxMode): TravelMode {
  switch (mode) {
    case TravelmuxMode.Walk:
      return TravelMode.Walk;
    case TravelmuxMode.Bike:
      return TravelMode.Bike;
    case TravelmuxMode.Drive:
      return TravelMode.Drive;
    case TravelmuxMode.Transit:
      return TravelMode.Transit;
  }
}
