export default class TripLayerId {
  tripIdx: number;
  legIdx: number;
  selected: boolean;

  constructor(tripIdx: number, legIdx: number, selected: boolean) {
    this.tripIdx = tripIdx;
    this.legIdx = legIdx;
    this.selected = selected;
  }

  static selected(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, true);
  }

  static unselected(tripIdx: number, legIdx: number): TripLayerId {
    return new TripLayerId(tripIdx, legIdx, false);
  }

  public toString(): string {
    const selectionState = this.selected ? 'selected' : 'unselected';
    return `trip_${this.tripIdx}_leg_${this.legIdx}_${selectionState}`;
  }
}
