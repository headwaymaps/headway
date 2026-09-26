enum LegPart {
  START = 'start',
  MIDDLE = 'middle',
  /// The rest of the transit route, beyond the part this leg rides.
  CONTEXT = 'context',
  /// A dot at every ordinary stop on the portion of the route the rider travels.
  RIDDEN_STOPS = 'ridden_stops',
  /// The same, for the stops beyond the part the rider is aboard for.
  CONTEXT_STOPS = 'context_stops',
  /// The stops where the rider boards and alights, drawn heavier than the rest.
  ON_OFF_STOPS = 'on_off_stops',
}

export default class TripLayerId {
  tripIdx: number;
  legIdx: number;
  selected: boolean;
  legPart: LegPart;

  constructor(
    tripIdx: number,
    legIdx: number,
    selected: boolean,
    legPart: LegPart,
  ) {
    this.tripIdx = tripIdx;
    this.legIdx = legIdx;
    this.selected = selected;
    this.legPart = legPart;
  }

  static selectedLeg(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.MIDDLE);
  }

  static unselectedLeg(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, false, LegPart.MIDDLE);
  }

  static legContext(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.CONTEXT);
  }

  static legRiddenStops(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.RIDDEN_STOPS);
  }

  static legContextStops(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.CONTEXT_STOPS);
  }

  static legOnOffStops(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.ON_OFF_STOPS);
  }

  static legStart(tripIdx: number, legIdx: number): TripLayerId {
    // Note: no difference between selected and unselected
    return new TripLayerId(tripIdx, legIdx, true, LegPart.START);
  }

  public toString(): string {
    const selectionState = this.selected ? 'selected' : 'unselected';
    return `trip_${this.tripIdx}_leg_${this.legIdx}_${selectionState}_${this.legPart}`;
  }
}
