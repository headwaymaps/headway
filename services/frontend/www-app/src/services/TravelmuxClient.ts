import { LngLat, LngLatBounds } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Ok, Result } from 'src/utils/Result';
import Trip, { TripFetchError, TripLeg } from 'src/models/Trip';
import { OTPPlanResponse, OTPItinerary } from './OTPClient';
import { ValhallaRouteResponse, ValhallaRoute } from './ValhallaClient';
import Itinerary from 'src/models/Itinerary';
import Route from 'src/models/Route';
import { zipWith } from 'lodash';
import { formatDuration } from 'src/utils/format';

export interface TravelmuxPlanResponse {
  _otp: OTPPlanResponse;
  _valhalla: ValhallaRouteResponse;
  plan: TravelmuxPlan;
}

export interface TravelmuxPlan {
  itineraries: TravelmuxItinerary[];
}
export interface TravelmuxItinerary {
  duration: number;
  // legs: TravelmuxLeg[];
  // startTime: number;
  // endTime: number;
  // walkDistance: number;
  // transitTime: number;
  // waitingTime: number;
  // walkTime: number;
  // transitTransfers: number;
  // elevationGained: number;
  // elevationLost: number;
  // fare: number;
  // legs: TravelmuxLeg[];
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
  // comma separated list of OTPMode(s)
  mode?: string;
};

export enum TravelmuxMode {
  Bike = 'BICYCLE',
  Walk = 'WALK',
  Drive = 'CAR',
  Transit = 'TRANSIT',
}

export class TravelmuxTrip implements Trip {
  raw: TravelmuxItinerary;
  inner: Trip;

  constructor(raw: TravelmuxItinerary, inner: Trip) {
    this.raw = raw;
    this.inner = inner;
  }

  // write a getter for durationFormatted that delgates to inner.durationFormatted
  get durationFormatted(): string {
    return formatDuration(this.raw.duration, 'shortform');
  }

  get lengthFormatted(): string | undefined {
    return this.inner.lengthFormatted;
  }

  get bounds(): LngLatBounds {
    return this.inner.bounds;
  }

  get legs(): TripLeg[] {
    return this.inner.legs;
  }

  get mode(): TravelMode {
    return this.inner.mode;
  }

  get startStopTimesFormatted(): string | undefined {
    return this.inner.startStopTimesFormatted;
  }

  get formattedFootDistance(): string | undefined {
    return this.inner.formattedFootDistance;
  }

  // TODO: this is valhalla specific
  get viaRoadsFormatted(): string | undefined {
    return this.inner.viaRoadsFormatted;
  }
}

export class TravelmuxClient {
  public static async fetchPlans(
    from: LngLat,
    to: LngLat,
    modes: TravelmuxMode[],
    numItineraries: number,
    time?: string,
    date?: string,
    arriveBy?: boolean,
  ): Promise<Result<TravelmuxTrip[], TripFetchError>> {
    const params: TravelmuxPlanRequest = {
      fromPlace: `${from.lat},${from.lng}`,
      toPlace: `${to.lat},${to.lng}`,
      numItineraries: `${numItineraries}`,
      mode: modes.join(','),
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

    const response = await fetch('/transitmux/v2/plan?' + query);

    if (response.ok) {
      // TODO: sort responses by arrival time like we did w/ OTPClient
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
              DistanceUnits.Kilometers, // FIXME: always km
              modes.includes(TravelmuxMode.Bike),
            );
            return new TravelmuxTrip(tmxItinerary, otpItinerary);
          },
        );
        return Ok(trips);
      } else if (travelmuxResponseJson._valhalla) {
        const routes: ValhallaRoute[] = [];
        if (travelmuxResponseJson._valhalla.trip) {
          routes.push(travelmuxResponseJson._valhalla.trip);
        }
        for (const route of travelmuxResponseJson._valhalla.alternates || []) {
          if (route.trip) {
            routes.push(route.trip);
          }
        }
        const trips = zipWith(
          tmxItineraries,
          routes,
          (tmxItinerary: TravelmuxItinerary, valhallaRoute: ValhallaRoute) => {
            console.assert(tmxItinerary, 'expected tmxItinerary to be set');
            console.assert(valhallaRoute, 'expected valhallaRoute to be set');
            const route = Route.fromValhalla(
              valhallaRoute,
              travelModeFromTravelmuxMode(modes[0]),
            );
            return new TravelmuxTrip(tmxItinerary, route);
          },
        );
        return Ok(trips);
      } else {
        throw Error('missing routing backend');
      }
    } else {
      if (modes.includes(TravelmuxMode.Transit)) {
        console.warn('Error in OTP response', response);
        // const responseError = { status: response.status };
        // TODO: handle OTPErrorId.
        throw new Error('Transit error handling not yet implemented');
      } else {
        //return Err({ transit: false, responseError });
        throw new Error('Non-transit error handling not yet implemented');
      }
    }
  }
}

function travelModeFromTravelmuxMode(mode: TravelmuxMode): TravelMode {
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
