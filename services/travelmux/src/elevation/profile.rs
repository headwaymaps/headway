use crate::otp::gtfs_graphql;

/// Elevation along a route, sampled at even intervals.
#[derive(Debug, Clone, PartialEq)]
pub struct ElevationProfile {
    /// How far along the route the first sample is.
    start_meters: f64,
    /// Meters above sea level, one every [`Self::SAMPLE_METERS`] from `start_meters`.
    elevations: Vec<f64>,
}

/// A stretch of a route that climbs or descends at [`ElevationProfile::STEEP_GRADE`] or more.
#[derive(Debug, Clone, PartialEq)]
pub struct SteepSection {
    pub start_meters: f64,
    pub end_meters: f64,
    /// Rise over run across the whole section: positive climbs, negative descends.
    pub average_grade: f64,
    /// The steepest grade within the section, signed like `average_grade`.
    pub max_grade: f64,
}

impl ElevationProfile {
    const SAMPLE_METERS: f64 = 10.0;
    /// Averaging elevations over this distance keeps a noisy DEM pixel from reading as a hill.
    const SMOOTHING_METERS: f64 = 30.0;
    pub const STEEP_GRADE: f64 = 0.05;
    const MIN_SECTION_METERS: f64 = 30.0;
    /// Steep stretches separated by less than this, like a hill broken up by flat cross streets,
    /// are reported as one section.
    const MERGE_GAP_METERS: f64 = 30.0;

    /// OTP measures each step's profile from the start of that step. Steps without a profile,
    /// like bridges OTP flattens, are interpolated across.
    pub fn from_otp_steps<'a>(
        steps: impl IntoIterator<Item = &'a gtfs_graphql::Step>,
    ) -> Option<Self> {
        let mut points: Vec<(f64, f64)> = vec![];
        let mut step_start = 0.0;
        for step in steps {
            let step_distance = step.distance.unwrap_or(0.0);
            for component in step.elevation_profile.iter().flatten().flatten() {
                let (Some(distance), Some(elevation)) = (component.distance, component.elevation)
                else {
                    continue;
                };
                // Where a trip starts or ends partway along a street, OTP can measure that step's
                // last points along the whole street, past the end of the step. They'd shadow the
                // next step's points.
                if distance > step_distance + 1.0 {
                    continue;
                }
                let distance = step_start + distance;
                if points.last().is_some_and(|(last, _)| distance <= *last) {
                    continue;
                }
                points.push((distance, elevation));
            }
            step_start += step_distance;
        }
        Self::resample(&points)
    }

    /// Linearly interpolates `points`, (distance, elevation) pairs in increasing distance, onto
    /// the sample grid.
    fn resample(points: &[(f64, f64)]) -> Option<Self> {
        let (first, last) = (points.first()?, points.last()?);
        if last.0 - first.0 < Self::SAMPLE_METERS {
            return None;
        }
        let sample_count = ((last.0 - first.0) / Self::SAMPLE_METERS).floor() as usize + 1;
        let mut segment = points.windows(2).peekable();
        let elevations = (0..sample_count)
            .map(|i| {
                let distance = first.0 + i as f64 * Self::SAMPLE_METERS;
                while let Some([_, end]) = segment.peek() {
                    if end.0 >= distance {
                        break;
                    }
                    segment.next();
                }
                let Some([start, end]) = segment.peek() else {
                    return last.1;
                };
                let fraction = (distance - start.0) / (end.0 - start.0);
                start.1 + fraction * (end.1 - start.1)
            })
            .collect();
        Some(Self {
            start_meters: first.0,
            elevations,
        })
    }

    /// `(meters along the route, smoothed elevation)` for each sample.
    pub fn smoothed_points(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.smoothed()
            .into_iter()
            .enumerate()
            .map(|(i, elevation)| (self.distance_at(i), elevation))
    }

    fn distance_at(&self, sample: usize) -> f64 {
        self.start_meters + sample as f64 * Self::SAMPLE_METERS
    }

    fn smoothed(&self) -> Vec<f64> {
        let radius = (Self::SMOOTHING_METERS / Self::SAMPLE_METERS / 2.0).floor() as usize;
        let len = self.elevations.len();
        (0..len)
            .map(|i| {
                let window = &self.elevations[i.saturating_sub(radius)..(i + radius + 1).min(len)];
                window.iter().sum::<f64>() / window.len() as f64
            })
            .collect()
    }

    /// The grade between each pair of consecutive samples.
    fn grades(&self) -> Vec<f64> {
        self.smoothed()
            .windows(2)
            .map(|pair| (pair[1] - pair[0]) / Self::SAMPLE_METERS)
            .collect()
    }

    /// Total meters climbed and descended.
    pub fn climb_and_fall(&self) -> (f64, f64) {
        self.grades()
            .iter()
            .fold((0.0, 0.0), |(climb, fall), grade| {
                let rise = grade * Self::SAMPLE_METERS;
                if rise > 0.0 {
                    (climb + rise, fall)
                } else {
                    (climb, fall - rise)
                }
            })
    }

    pub fn steep_sections(&self) -> Vec<SteepSection> {
        let grades = self.grades();
        let smoothed = self.smoothed();

        // Runs of consecutive sample intervals steep in the same direction, as index ranges.
        let mut runs: Vec<(usize, usize)> = vec![];
        let mut run_start: Option<usize> = None;
        for (i, grade) in grades.iter().enumerate() {
            let continues = run_start.is_some_and(|start| {
                grade.abs() >= Self::STEEP_GRADE && grade.signum() == grades[start].signum()
            });
            if continues {
                continue;
            }
            if let Some(start) = run_start.take() {
                runs.push((start, i));
            }
            if grade.abs() >= Self::STEEP_GRADE {
                run_start = Some(i);
            }
        }
        if let Some(start) = run_start {
            runs.push((start, grades.len()));
        }

        let max_gap = (Self::MERGE_GAP_METERS / Self::SAMPLE_METERS) as usize;
        let mut merged: Vec<(usize, usize)> = vec![];
        for (start, end) in runs {
            match merged.last_mut() {
                Some(last)
                    if start - last.1 < max_gap
                        && grades[start].signum() == grades[last.0].signum() =>
                {
                    last.1 = end
                }
                _ => merged.push((start, end)),
            }
        }

        merged
            .into_iter()
            .filter_map(|(start, end)| {
                let length = (end - start) as f64 * Self::SAMPLE_METERS;
                if length < Self::MIN_SECTION_METERS {
                    return None;
                }
                let max_grade = grades[start..end]
                    .iter()
                    .copied()
                    .max_by(|a, b| a.abs().total_cmp(&b.abs()))?;
                Some(SteepSection {
                    start_meters: self.distance_at(start),
                    end_meters: self.distance_at(end),
                    average_grade: (smoothed[end] - smoothed[start]) / length,
                    max_grade,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    /// A profile through `(distance, elevation)` corners, joined by straight lines.
    fn profile(corners: &[(f64, f64)]) -> ElevationProfile {
        ElevationProfile::resample(corners).unwrap()
    }

    fn step(distance: f64, profile: &[(f64, f64)]) -> gtfs_graphql::Step {
        gtfs_graphql::Step {
            distance: Some(distance),
            relative_direction: None,
            absolute_direction: None,
            street_name: None,
            lat: None,
            lon: None,
            area: None,
            bogus_name: None,
            stay_on: None,
            exit: None,
            elevation_profile: Some(
                profile
                    .iter()
                    .map(|&(distance, elevation)| {
                        Some(gtfs_graphql::ElevationProfileComponent {
                            distance: Some(distance),
                            elevation: Some(elevation),
                        })
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn ignores_points_past_the_end_of_their_step() {
        // As OTP sent it for a trip starting partway along 25th Ave, before a steady 6% climb.
        let steps = [
            step(
                115.0,
                &[(0.0, 87.0), (106.0, 86.1), (212.0, 86.1), (221.0, 86.1)],
            ),
            step(100.0, &[(0.0, 86.1), (50.0, 89.1), (100.0, 92.1)]),
        ];
        let profile = ElevationProfile::from_otp_steps(&steps).unwrap();
        let steepest = profile
            .steep_sections()
            .iter()
            .map(|section| section.max_grade)
            .fold(0.0, f64::max);
        assert_relative_eq!(steepest, 0.06, epsilon = 0.005);
    }

    #[test]
    fn flat_route_has_no_steep_sections() {
        let flat = profile(&[(0.0, 10.0), (500.0, 10.0)]);
        assert_eq!(flat.steep_sections(), vec![]);
        assert_eq!(flat.climb_and_fall(), (0.0, 0.0));
    }

    #[test]
    fn single_climb() {
        // flat, then 10% for 200m, then flat
        let hill = profile(&[(0.0, 0.0), (200.0, 0.0), (400.0, 20.0), (600.0, 20.0)]);
        let sections = hill.steep_sections();
        assert_eq!(sections.len(), 1);
        let climb = &sections[0];
        // smoothing softens the corners
        assert_relative_eq!(climb.start_meters, 200.0, epsilon = 10.0);
        assert_relative_eq!(climb.end_meters, 400.0, epsilon = 10.0);
        assert_relative_eq!(climb.max_grade, 0.1, epsilon = 1e-9);
        assert_relative_eq!(climb.average_grade, 0.1, epsilon = 0.005);

        let (climb, fall) = hill.climb_and_fall();
        assert_relative_eq!(climb, 20.0, epsilon = 1e-9);
        assert_relative_eq!(fall, 0.0);
    }

    #[test]
    fn descent_is_negative() {
        let hill = profile(&[(0.0, 30.0), (300.0, 0.0)]);
        let sections = hill.steep_sections();
        assert_eq!(sections.len(), 1);
        assert_relative_eq!(sections[0].max_grade, -0.1, epsilon = 1e-9);
    }

    #[test]
    fn short_bump_is_ignored() {
        // 2m up and back down over 20m - steep, but too short to mention
        let bump = profile(&[
            (0.0, 0.0),
            (100.0, 0.0),
            (110.0, 2.0),
            (120.0, 0.0),
            (300.0, 0.0),
        ]);
        assert_eq!(bump.steep_sections(), vec![]);
    }

    #[test]
    fn hill_broken_by_flat_cross_street_is_one_section() {
        let hill = profile(&[
            (0.0, 0.0),
            (100.0, 10.0),
            (120.0, 10.0),
            (220.0, 20.0),
            (300.0, 20.0),
        ]);
        assert_eq!(hill.steep_sections().len(), 1);
    }

    #[test]
    fn climb_then_descent_are_separate() {
        let ridge = profile(&[(0.0, 0.0), (200.0, 20.0), (400.0, 0.0)]);
        let sections = ridge.steep_sections();
        assert_eq!(sections.len(), 2);
        assert!(sections[0].average_grade > 0.0);
        assert!(sections[1].average_grade < 0.0);
    }
}
