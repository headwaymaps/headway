// incomplete
export interface ValhallaRouteLegManeuver {
  begin_shape_index: number;
  end_shape_index: number;
  street_names?: string[];
  travel_mode: ValhallaTravelMode;
  time: number;
  cost: number;
  length: number;
  instruction: string;
  verbal_post_transition_instruction: string;
  type: number;
}

// incomplete
export enum ValhallaErrorCode {
  UnsupportedArea = 171,
  Unknown,
}

export type ValhallaError = {
  error_code: ValhallaErrorCode;
  error: string;
  status_code: number;
  status: string;
};

// incomplete
export interface ValhallaRouteSummary {
  time: number;
  length: number;
  min_lat: number;
  min_lon: number;
  max_lat: number;
  max_lon: number;
}

// From https://github.com/valhalla/valhalla-docs/blob/master/turn-by-turn/api-reference.md
// See also: travel_type for transit sub types like tram, ferry, etc.
export enum ValhallaTravelMode {
  Bicycle = 'bicycle',
  Drive = 'drive',
  Transit = 'transit',
  Walk = 'pedestrian',
}

// incomplete
export interface ValhallaRouteLeg {
  maneuvers: ValhallaRouteLegManeuver[];
  shape: string;
}

export interface ValhallaRouteResponse {
  trip: ValhallaRoute;
  alternates?: { trip: ValhallaRoute }[];
}

// incomplete
export interface ValhallaRoute {
  legs: ValhallaRouteLeg[];
  summary: ValhallaRouteSummary;
  units: string;
}

export function valhallaTypeToIcon(type: number) {
  switch (type) {
    case 1:
    case 2:
    case 3:
      return 'straight';
    case 4:
    case 5:
    case 6:
      return 'place';
    case 7:
    case 8:
      return 'straight';
    case 9:
      return 'turn_slight_right';
    case 10:
      return 'turn_right';
    case 11:
      return 'turn_sharp_right';
    case 12:
      return 'u_turn_right';
    case 13:
      return 'u_turn_left';
    case 14:
      return 'turn_sharp_left';
    case 15:
      return 'turn_left';
    case 16:
      return 'turn_slight_left';
    case 17:
      return 'straight';
    case 18:
      return 'turn_slight_right';
    case 19:
      return 'turn_slight_left';
    case 20:
      return 'turn_slight_right';
    case 21:
      return 'turn_slight_left';
    case 22:
      return 'straight';
    case 23:
      return 'turn_slight_right';
    case 24:
      return 'turn_slight_left';
    case 25:
      return 'merge';
    case 26:
      return 'login';
    case 27:
      return 'logout';
    case 28:
      return 'login';
    case 29:
      return 'logout';
  }
  return '';
}

export type CacheableMode = 'walk' | 'bicycle' | 'car';
