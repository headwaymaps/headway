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
  realTime: boolean;
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
  // comma separated list of OTPMode(s)
  mode?: string;
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

export type OTPResponseError = { status: number };
