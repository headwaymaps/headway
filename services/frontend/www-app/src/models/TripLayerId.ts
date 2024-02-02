enum LegPart {
  START = 'start',
  MIDDLE = 'middle',
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

  static legStart(tripIdx: number, legIdx: number): TripLayerId {
    // Note: no difference between selected and unselected
    return new TripLayerId(tripIdx, legIdx, true, LegPart.START);
  }

  public toString(): string {
    const selectionState = this.selected ? 'selected' : 'unselected';
    return `trip_${this.tripIdx}_leg_${this.legIdx}_${selectionState}_${this.legPart}`;
  }
}
