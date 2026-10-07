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

const MAX_ANNOTATED_CLIMBS = 3;
const MIN_ANNOTATED_CLIMB_SPACING_METERS = 100;

/// The steepest few climbs, spread out enough that their signs don't crowd each other.
export function annotatedSteepClimbs(sections: SteepSection[]): SteepSection[] {
  const steepestFirst = sections
    .filter((section) => section.averageGrade > 0)
    .sort((a, b) => b.averageGrade - a.averageGrade);
  const annotated: SteepSection[] = [];
  for (const section of steepestFirst) {
    if (annotated.length === MAX_ANNOTATED_CLIMBS) {
      break;
    }
    const isSpacedOut = annotated.every(
      (other) =>
        Math.max(
          section.startMeters - other.endMeters,
          other.startMeters - section.endMeters,
        ) >= MIN_ANNOTATED_CLIMB_SPACING_METERS,
    );
    if (isSpacedOut) {
      annotated.push(section);
    }
  }
  return annotated;
}

/// Ground that isn't steep enough to be a `SteepSection`.
export const FLAT_GRADE_COLOR = '#43A047';

/// How a steep stretch is drawn: climbs in warm colors and descents in deep greens, by how steep.
export enum GradeShade {
  ModerateClimb = 'moderate_climb',
  SteepClimb = 'steep_climb',
  VerySteepClimb = 'very_steep_climb',
  ModerateDescent = 'moderate_descent',
  SteepDescent = 'steep_descent',
}

export const GradeShades = Object.values(GradeShade);

export function gradeShade(grade: number): GradeShade {
  const tier = gradeTier(grade);
  if (grade < 0) {
    return tier === GradeTier.Moderate
      ? GradeShade.ModerateDescent
      : GradeShade.SteepDescent;
  }
  switch (tier) {
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
    case GradeShade.ModerateDescent:
      return '#2E7D32';
    case GradeShade.SteepDescent:
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

/// e.g. "↗ 14%"
export function formatClimbGrade(grade: number): string {
  return `↗ ${Math.round(grade * 100)}%`;
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
