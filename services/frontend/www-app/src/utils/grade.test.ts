import { describe, expect, test } from 'vitest';
import { SteepSection } from 'src/services/TravelmuxClient';
import {
  describeGrade,
  GradeShade,
  gradeShade,
  midpointFraction,
} from './grade';

function section(
  startMeters: number,
  endMeters: number,
  averageGrade: number,
): SteepSection {
  return {
    startMeters,
    endMeters,
    averageGrade,
    maxGrade: averageGrade,
    geometry: '',
    annotated: false,
  };
}

describe('describeGrade', () => {
  test('names the tier', () => {
    expect(describeGrade(0.06)).toBe('6% moderate grade');
    expect(describeGrade(0.1)).toBe('10% steep grade');
    expect(describeGrade(0.14)).toBe('14% very steep grade');
  });
});

describe('gradeShade', () => {
  test('shades climbs and descents by steepness', () => {
    expect(gradeShade(0.06)).toBe(GradeShade.ModerateClimb);
    expect(gradeShade(0.14)).toBe(GradeShade.VerySteepClimb);
    expect(gradeShade(-0.14)).toBe(GradeShade.VerySteepDescent);
  });
});

describe('midpointFraction', () => {
  test('measures from where the profile starts', () => {
    const profile: [number, number][] = [
      [100, 0],
      [500, 0],
    ];
    expect(midpointFraction(profile, section(200, 300, 0.1))).toBe(0.375);
  });
});
