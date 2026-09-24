/// Why a vehicle marker's popover is showing.
export enum PopoverState {
  Closed = 'closed',
  Hovered = 'hovered',
  Pinned = 'pinned',
}

/// The popover on a transit vehicle marker: hovering previews it, a click pins it open.
///
/// A pinned popover ignores the pointer entirely - the marker crawls out from under a still
/// pointer as the vehicle moves, which would otherwise take the popover with it.
export default class VehiclePopover {
  state: PopoverState = PopoverState.Closed;

  get isOpen(): boolean {
    return this.state !== PopoverState.Closed;
  }

  get isPinned(): boolean {
    return this.state === PopoverState.Pinned;
  }

  pointerEntered(): void {
    if (this.state === PopoverState.Closed) {
      this.state = PopoverState.Hovered;
    }
  }

  pointerLeft(): void {
    if (this.state === PopoverState.Hovered) {
      this.state = PopoverState.Closed;
    }
  }

  /// Follows the overlay, which pins at most one vehicle at a time.
  setPinned(pinned: boolean): void {
    this.state = pinned ? PopoverState.Pinned : PopoverState.Closed;
  }
}
