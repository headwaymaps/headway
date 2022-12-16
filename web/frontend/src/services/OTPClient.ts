import { LngLat } from 'maplibre-gl';
import { Err, Ok, Result } from 'src/utils/Result';

// incomplete
export type OTPLegGeometry = {
  points: string;
};

export enum OTPMode {
  Walk = 'WALK',
  Bus = 'BUS',
  Train = 'TRAIN',
  Tram = 'TRAM',
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
  routeShortName?: string;
  from: { name: string; lat: number; lon: number };
  to: { name: string; lat: number; lon: number };
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
    count: number
  ): Promise<Result<OTPItinerary[], OTPError>> {
    const response = await fetch(
      `/otp/routers/default/plan?fromPlace=${from.lat},${from.lng}&toPlace=${to.lat},${to.lng}&numItineraries=${count}`
    );
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
