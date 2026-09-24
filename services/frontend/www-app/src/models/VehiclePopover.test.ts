import { describe, expect, test } from 'vitest';
import VehiclePopover, { PopoverState } from './VehiclePopover';

describe('VehiclePopover', () => {
  test('hovering previews it and moving away closes it', () => {
    const popover = new VehiclePopover();
    expect(popover.isOpen).toBe(false);

    popover.pointerEntered();
    expect(popover.state).toBe(PopoverState.Hovered);

    popover.pointerLeft();
    expect(popover.state).toBe(PopoverState.Closed);
  });

  test('a pin outlasts the pointer', () => {
    const popover = new VehiclePopover();
    popover.pointerEntered();
    popover.setPinned(true);

    popover.pointerLeft();
    expect(popover.state).toBe(PopoverState.Pinned);

    popover.pointerEntered();
    popover.pointerLeft();
    expect(popover.state).toBe(PopoverState.Pinned);
  });

  test('releasing the pin closes it', () => {
    const popover = new VehiclePopover();
    popover.setPinned(true);
    popover.setPinned(false);
    expect(popover.state).toBe(PopoverState.Closed);
  });
});
