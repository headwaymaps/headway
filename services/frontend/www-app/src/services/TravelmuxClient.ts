import { LineLayerSpecification, LngLat, LngLatBounds } from 'maplibre-gl';
import { DistanceUnits, TravelMode } from 'src/utils/models';
import { Ok, Result } from 'src/utils/Result';
import { LineStyles, TripFetchError } from 'src/models/Trip';
import { OTPPlanResponse, OTPItinerary } from './OTPClient';
import { ValhallaRouteResponse, ValhallaRoute } from './ValhallaClient';
import Itinerary from 'src/models/Itinerary';
import Route from 'src/models/Route';
import { zipWith } from 'lodash';
import { formatDistance, formatDuration } from 'src/utils/format';
import { LineString } from 'geojson';
import { decodePolyline } from 'src/third_party/decodePath';

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
  routeColor?: string;
}

export interface TravelmuxItinerary {
  mode: TravelmuxMode;
  duration: number;
  distance: number;
  distanceUnits: DistanceUnits;
  bounds: { min: [number, number]; max: [number, number] };
  legs: TravelmuxLeg[];
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

export class TravelmuxTripLeg {
  raw: TravelmuxLeg;
  geometry: LineString;

  constructor(raw: TravelmuxLeg) {
    this.raw = raw;
    const points = decodePolyline(this.raw.geometry, 6, false);
    this.geometry = {
      type: 'LineString',
      coordinates: points,
    };
  }

  get start(): LngLat {
    const lngLat = this.geometry.coordinates[0];
    return new LngLat(lngLat[0], lngLat[1]);
  }

  get mode(): TravelMode {
    return travelModeFromTravelmuxMode(this.raw.mode);
  }

  paintStyle(active: boolean): LineLayerSpecification['paint'] {
    if (active) {
      if (this.mode == TravelMode.Walk || this.mode == TravelMode.Bike) {
        return LineStyles.walkingActive;
      } else {
        if (this.raw.routeColor) {
          return LineStyles.activeColored(`#${this.raw.routeColor}`);
        } else {
          return LineStyles.active;
        }
      }
    } else {
      if (this.mode == TravelMode.Walk || this.mode == TravelMode.Bike) {
        return LineStyles.walkingInactive;
      } else {
        return LineStyles.inactive;
      }
    }
  }
}

export class TravelmuxTrip {
  raw: TravelmuxItinerary;
  inner: Route | Itinerary;
  preferredDistanceUnits: DistanceUnits;
  innerDistanceUnits: DistanceUnits;

  constructor(
    raw: TravelmuxItinerary,
    preferredDistanceUnits: DistanceUnits,
    inner: Route | Itinerary,
    innerDistanceUnits: DistanceUnits,
  ) {
    this.raw = raw;
    this.preferredDistanceUnits = preferredDistanceUnits;
    this.inner = inner;
    this.innerDistanceUnits = innerDistanceUnits;
  }

  get durationFormatted(): string {
    return formatDuration(this.raw.duration, 'shortform');
  }

  get distanceFormatted(): string | undefined {
    return formatDistance(
      this.raw.distance,
      this.innerDistanceUnits,
      this.preferredDistanceUnits,
    );
  }

  get bounds(): LngLatBounds {
    return new LngLatBounds(this.raw.bounds.min, this.raw.bounds.max);
  }

  get legs(): TravelmuxTripLeg[] {
    return this.raw.legs.map((raw: TravelmuxLeg) => new TravelmuxTripLeg(raw));
  }

  get mode(): TravelMode {
    return travelModeFromTravelmuxMode(this.raw.mode);
  }

  transitItinerary(): Itinerary | undefined {
    if (this.mode == TravelMode.Transit) {
      return this.inner as Itinerary;
    } else {
      return undefined;
    }
  }

  nonTransitRoute(): Route | undefined {
    if (this.mode != TravelMode.Transit) {
      return this.inner as Route;
    } else {
      return undefined;
    }
  }
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
  ): Promise<Result<TravelmuxTrip[], TripFetchError>> {
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

    const response = await fetch('/travelmux/v2/plan?' + query);

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
              preferredDistanceUnits,
              modes.includes(TravelmuxMode.Bike),
            );
            // OTP always returns metric units
            return new TravelmuxTrip(
              tmxItinerary,
              preferredDistanceUnits,
              otpItinerary,
              DistanceUnits.Kilometers,
            );
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
              preferredDistanceUnits,
            );
            console.assert(
              preferredDistanceUnits == tmxItinerary.distanceUnits,
              'expected preferredDistanceUnits to match tmxItinerary.distanceUnits for valhalla requests',
            );
            return new TravelmuxTrip(
              tmxItinerary,
              preferredDistanceUnits,
              route,
              tmxItinerary.distanceUnits,
            );
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
