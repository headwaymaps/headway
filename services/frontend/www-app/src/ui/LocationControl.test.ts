import LocationControl from './LocationControl';

describe('LocationControl', () => {
  describe('nearestDelta', () => {
    it('should calculate the shortest distance between two angles', () => {
      expect(LocationControl.nearestDelta(0, 90)).toBe(90);
      expect(LocationControl.nearestDelta(90, 0)).toBe(-90);
      expect(LocationControl.nearestDelta(180, 270)).toBe(90);
      expect(LocationControl.nearestDelta(270, 180)).toBe(-90);
      expect(LocationControl.nearestDelta(0, 360)).toBe(0);
      expect(LocationControl.nearestDelta(360, 0)).toBe(-0);
      expect(LocationControl.nearestDelta(359, 1)).toBe(2);
      expect(LocationControl.nearestDelta(1, 359)).toBe(-2);
      expect(LocationControl.nearestDelta(5 + 360 * 2, 6)).toBe(1);
    });
  });
});
