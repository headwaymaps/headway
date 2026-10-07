import { describe, expect, test } from 'vitest';
import { SteepSection } from 'src/services/TravelmuxClient';
import {
  annotatedSteepClimbs,
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
  };
}

describe('annotatedSteepClimbs', () => {
  test('keeps the three steepest climbs, ignoring descents', () => {
    const sections = [
      section(0, 100, 0.06),
      section(200, 300, 0.1),
      section(400, 500, -0.2),
      section(600, 700, 0.07),
      section(800, 900, 0.14),
    ];
    expect(annotatedSteepClimbs(sections)).toEqual([
      sections[4],
      sections[1],
      sections[3],
    ]);
  });

  test('skips a climb within 100m of a steeper one', () => {
    const sections = [
      section(0, 100, 0.14),
      section(150, 250, 0.12),
      section(300, 400, 0.06),
    ];
    expect(annotatedSteepClimbs(sections)).toEqual([sections[0], sections[2]]);
  });
});

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
    expect(gradeShade(-0.06)).toBe(GradeShade.ModerateDescent);
    expect(gradeShade(-0.1)).toBe(GradeShade.SteepDescent);
    expect(gradeShade(-0.14)).toBe(GradeShade.SteepDescent);
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
