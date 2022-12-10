import { LngLat } from 'maplibre-gl';

export type OTPLegGeometry = {
  points: string;
};

export enum OTPMode {
  Walk = 'WALK',
  Bus = 'BUS',
  Train = 'TRAIN',
  Tram = 'TRAM',
}

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

export type OTPItinerary = {
  generalizedCost: number;
  duration: number;
  startTime: number;
  endTime: number;
  walkDistance: number;
  legs: OTPItineraryLeg[];
};

export class OTPClient {
  public static async fetchItineraries(
    from: LngLat,
    to: LngLat,
    count: number
  ): Promise<OTPItinerary[]> {
    const rawResponse = await fetch(
      `/otp/routers/default/plan?fromPlace=${from.lat},${from.lng}&toPlace=${to.lat},${to.lng}&numItineraries=${count}`
    );
    const response = await rawResponse.json();
    return response.plan.itineraries.sort(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (a: any, b: any) => a.endTime - b.endTime
    );
  }
}
