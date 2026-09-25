enum LegPart {
  START = 'start',
  MIDDLE = 'middle',
  /// The rest of the transit route, beyond the part this leg rides.
  CONTEXT = 'context',
  /// A dot at every stop the transit route calls at.
  STOPS = 'stops',
  /// The same, for the stops beyond the part the rider is aboard for.
  CONTEXT_STOPS = 'context_stops',
  /// The stops this leg boards and alights at, drawn heavier than the rest.
  USED_STOPS = 'used_stops',
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

  static legStops(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.STOPS);
  }

  static legContextStops(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.CONTEXT_STOPS);
  }

  static legUsedStops(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true, LegPart.USED_STOPS);
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
