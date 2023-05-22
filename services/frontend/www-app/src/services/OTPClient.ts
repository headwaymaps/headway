import { LngLat } from 'maplibre-gl';
import { Err, Ok, Result } from 'src/utils/Result';

// incomplete
export type OTPLegGeometry = {
  points: string;
};

export enum OTPMode {
  Bicycle = 'BICYCLE',
  Bus = 'BUS',
  CableCar = 'CABLE_CAR',
  Car = 'CAR',
  Ferry = 'FERRY',
  Funicular = 'FUNICULAR',
  Gondola = 'GONDOLA',
  Rail = 'RAIL',
  Subway = 'SUBWAY',
  Train = 'TRAIN',
  Tram = 'TRAM',
  Transit = 'TRANSIT',
  Walk = 'WALK',
}

// incomplete
export enum OTPErrorId {
  OutsideBounds = 400,
  TooClose = 409,
}

// incomplete
export type OTPPlanError = {
  id: OTPErrorId;
  msg: string;
  message: string;
  missing: string[];
};

// incomplete
export type OTPItineraryLeg = {
  startTime: number;
  endTime: number;
  mode: OTPMode;
  transitLeg: boolean;
  legGeometry: OTPLegGeometry;
  // Seems to always be set, but may be an empty string, e.g. for a walk
  route: string;
  routeShortName?: string;
  routeLongName?: string;
  routeColor?: string;
  from: { name: string; lat: number; lon: number };
  to: { name: string; lat: number; lon: number };
  alerts: OTPAlert[];
};

export type OTPAlert = {
  alertHeaderText: string;
  alertDescriptionText: string;
  alertUrl: string;
  //	null means unknown
  effectiveStartDate: number | null;
  effectiveEndDate: number;
};

// incomplete
export type OTPItinerary = {
  generalizedCost: number;
  duration: number;
  startTime: number;
  endTime: number;
  walkDistance: number;
  legs: OTPItineraryLeg[];
};

// incomplete
export type OTPPlanRequest = {
  fromPlace: string;
  toPlace: string;
  // It'd be nice to typecheck this as numeric, but it would require some
  // additional type juggling elsewhere
  //numItineraries?: number,
  numItineraries?: string;
  time?: string;
  date?: string;
  arriveBy?: string;
};

// incomplete
export type OTPPlanResponse = {
  plan: {
    itineraries: OTPItinerary[];
  };
  error?: OTPPlanError;
};

export type OTPError =
  | { planError: OTPPlanError }
  | { responseError: OTPResponseError };

type OTPResponseError = { status: number };

export class OTPClient {
  public static async fetchItineraries(
    from: LngLat,
    to: LngLat,
    count: number,
    time?: string,
    date?: string,
    arriveBy?: boolean
  ): Promise<Result<OTPItinerary[], OTPError>> {
    const params: OTPPlanRequest = {
      fromPlace: `${from.lat},${from.lng}`,
      toPlace: `${to.lat},${to.lng}`,
      numItineraries: `${count}`,
    };

    // The OTP API assumes current date and time if neither are specified.
    // If only date is specified, the current time at that date is assumed.
    // If only time is specified, it's an error.
    if (time) {
      console.assert(
        date,
        'The OTP API requires that if time is specified, date must also be specified'
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

    const response = await fetch('/transitmux/plan?' + query);
    if (response.ok) {
      const responseJson: OTPPlanResponse = await response.json();
      if (responseJson.plan.itineraries.length > 0) {
        const itineraries = responseJson.plan.itineraries.sort(
          (a, b) => a.endTime - b.endTime
        );
        return Ok(itineraries);
      } else {
        if (responseJson.error) {
          return Err({ planError: responseJson.error });
        } else {
          console.error('Uknown error in OK OTP response', responseJson);
          throw new Error('Uknown error in OK OTP response');
        }
      }
    } else {
      console.warn('Error in OTP response', response);
      const responseError = { status: response.status };
      return Err({ responseError });
    }
  }
}
