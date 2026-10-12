import type { SteepSection } from 'src/services/TravelmuxClient';
import { i18n } from 'src/i18n/lang';

export enum GradeTier {
  Moderate = 'moderate',
  Steep = 'steep',
  VerySteep = 'very_steep',
}

export function gradeTier(grade: number): GradeTier {
  const steepness = Math.abs(grade);
  if (steepness >= 0.13) {
    return GradeTier.VerySteep;
  } else if (steepness >= 0.09) {
    return GradeTier.Steep;
  } else {
    return GradeTier.Moderate;
  }
}

/// Ground that isn't steep enough to be a `SteepSection`.
export const FLAT_GRADE_COLOR = '#43A047';

/// How a steep stretch is drawn: climbs in warm colors by how steep, and descents in deep green.
/// travelmux only sends descents that are very steep.
export enum GradeShade {
  ModerateClimb = 'moderate_climb',
  SteepClimb = 'steep_climb',
  VerySteepClimb = 'very_steep_climb',
  VerySteepDescent = 'very_steep_descent',
}

export const GradeShades = Object.values(GradeShade);

export function gradeShade(grade: number): GradeShade {
  if (grade < 0) {
    return GradeShade.VerySteepDescent;
  }
  switch (gradeTier(grade)) {
    case GradeTier.Moderate:
      return GradeShade.ModerateClimb;
    case GradeTier.Steep:
      return GradeShade.SteepClimb;
    case GradeTier.VerySteep:
      return GradeShade.VerySteepClimb;
  }
}

export function gradeShadeColor(shade: GradeShade): string {
  switch (shade) {
    case GradeShade.ModerateClimb:
      return '#F2B705';
    case GradeShade.SteepClimb:
      return '#F27405';
    case GradeShade.VerySteepClimb:
      return '#D92B04';
    case GradeShade.VerySteepDescent:
      return '#1B5E20';
  }
}

export function gradeColor(grade: number): string {
  return gradeShadeColor(gradeShade(grade));
}

/// e.g. "12% steep grade"
export function describeGrade(grade: number): string {
  return i18n.global.t(`${gradeTier(grade)}_grade_$grade`, {
    grade: `${Math.round(Math.abs(grade) * 100)}%`,
  });
}

/// e.g. "↗ 14%", or "↘ 14%" descending
export function formatSteepGrade(grade: number): string {
  return `${grade > 0 ? '↗' : '↘'} ${Math.round(Math.abs(grade) * 100)}%`;
}

/// How far along `profile` the middle of `section` is, as a fraction.
export function midpointFraction(
  profile: [number, number][],
  section: SteepSection,
): number {
  const start = profile[0]![0];
  const end = profile[profile.length - 1]![0];
  return (
    ((section.startMeters + section.endMeters) / 2 - start) / (end - start)
  );
}
