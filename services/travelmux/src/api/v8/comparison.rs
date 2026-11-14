//! Where the routes several routers planned for one trip agree and diverge.

use std::f64::consts::PI;
use std::ops::Range;

use geo::{coord, Coord, Distance, Euclidean, Line, LineString, Point, Simplify};
use serde::Serialize;

use super::Itinerary;
use crate::util::serde_util::serialize_line_strings_as_polyline6;

const SAMPLE_SPACING_METERS: f64 = 2.0;

/// How close a sample must come to another route to be on it. Routers put a way on its OSM nodes,
/// so the same way lines up within a meter, while a bike lane beside a road is further off.
const TOLERANCE_METERS: f64 = 3.0;

/// How far a sample's heading can differ from the other route's, so crossing a route isn't sharing it.
const MAX_HEADING_DIFFERENCE_DEGREES: f64 = 30.0;

/// Agreements or differences within a route shorter than this are noise, like where two routes
/// round a corner through slightly different vertices.
const MIN_STRETCH_METERS: f64 = 10.0;

/// Differences at either end up to this long are where routers snapped the trip's ends differently.
const MAX_END_SNAP_METERS: f64 = 25.0;

const METERS_PER_DEGREE_LAT: f64 = 111_320.0;

/// Drops the collinear vertices sampling adds, a centimeter or so off the line.
const SIMPLIFY_EPSILON_DEGREES: f64 = 1e-7;

/// The routes a trip's itineraries took, merging those from different routers on the same streets.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    /// On every route.
    #[serde(serialize_with = "serialize_line_strings_as_polyline6")]
    common: Vec<LineString>,
    routes: Vec<ComparedRoute>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComparedRoute {
    /// The indices of the itineraries that took this route, each from a different router.
    itineraries: Vec<usize>,
    /// On this route alone.
    #[serde(serialize_with = "serialize_line_strings_as_polyline6")]
    solo: Vec<LineString>,
    /// On some of the other routes too.
    #[serde(serialize_with = "serialize_line_strings_as_polyline6")]
    partly_shared: Vec<LineString>,
}

/// How many of the other routes a stretch of one is on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sharing {
    Every,
    Some,
    None,
}

impl Comparison {
    pub fn new(itineraries: &[Itinerary]) -> Self {
        let routes: Vec<(&str, LineString)> = itineraries
            .iter()
            .map(|itinerary| (itinerary.router_id.as_str(), itinerary.combined_geometry()))
            .collect();
        Self::of(&routes)
    }

    /// Compares each router's route, as `(router_id, geometry)`.
    fn of(routes: &[(&str, LineString)]) -> Self {
        let Some(origin) = routes.iter().find_map(|(_, line)| line.0.first()) else {
            return Self {
                common: vec![],
                routes: vec![],
            };
        };
        let projection = Projection::new(*origin);
        let sampled: Vec<SampledRoute> = routes
            .iter()
            .map(|(_, line)| SampledRoute::new(line, &projection))
            .collect();
        // on[i][j][s]: whether route i's sample s is on route j.
        let on: Vec<Vec<Vec<bool>>> = sampled
            .iter()
            .map(|route| {
                sampled
                    .iter()
                    .map(|other| route.samples_on(other))
                    .collect()
            })
            .collect();

        let mut groups: Vec<Vec<usize>> = vec![];
        for i in 0..routes.len() {
            let same_route = groups.iter_mut().find(|group| {
                let first = group[0];
                group.iter().all(|&j| routes[j].0 != routes[i].0)
                    && !strays(&on[i][first])
                    && !strays(&on[first][i])
            });
            match same_route {
                Some(group) => group.push(i),
                None => groups.push(vec![i]),
            }
        }

        let representatives: Vec<usize> = groups.iter().map(|group| group[0]).collect();
        let sharing = |route: usize, sample: usize| {
            let others = representatives.len() - 1;
            let on_others = representatives
                .iter()
                .filter(|&&other| other != route && on[route][other][sample])
                .count();
            if on_others == others {
                Sharing::Every
            } else if on_others == 0 {
                Sharing::None
            } else {
                Sharing::Some
            }
        };

        let common = sampled[representatives[0]]
            .stretches(|sample| sharing(representatives[0], sample) == Sharing::Every);
        let routes = groups
            .into_iter()
            .map(|itineraries| {
                let route = itineraries[0];
                ComparedRoute {
                    solo: sampled[route].stretches(|s| sharing(route, s) == Sharing::None),
                    partly_shared: sampled[route].stretches(|s| sharing(route, s) == Sharing::Some),
                    itineraries,
                }
            })
            .collect();
        Self { common, routes }
    }
}

/// A flat projection in meters around an origin, accurate enough across a city.
struct Projection {
    meters_per_degree_lng: f64,
}

impl Projection {
    fn new(origin: Coord) -> Self {
        Self {
            meters_per_degree_lng: METERS_PER_DEGREE_LAT * origin.y.to_radians().cos(),
        }
    }

    fn project(&self, lng_lat: Coord) -> Coord {
        coord! { x: lng_lat.x * self.meters_per_degree_lng, y: lng_lat.y * METERS_PER_DEGREE_LAT }
    }
}

/// A route sampled every few meters, so routes can be compared along their length.
struct SampledRoute {
    /// Each sample, in degrees.
    lng_lats: Vec<Coord>,
    /// Each sample, projected.
    points: Vec<Coord>,
    /// Which way the route heads at each sample, in radians.
    headings: Vec<f64>,
    /// The route's segments, projected.
    segments: Vec<Line>,
}

impl SampledRoute {
    fn new(line: &LineString, projection: &Projection) -> Self {
        let mut route = Self {
            lng_lats: vec![],
            points: vec![],
            headings: vec![],
            segments: vec![],
        };
        for segment in line.lines() {
            let projected = Line::new(
                projection.project(segment.start),
                projection.project(segment.end),
            );
            let length = projected.dx().hypot(projected.dy());
            if length == 0.0 {
                continue;
            }
            let heading = projected.dy().atan2(projected.dx());
            if route.lng_lats.is_empty() {
                route.push(segment.start, projected.start, heading);
            }
            let steps = (length / SAMPLE_SPACING_METERS).ceil();
            for step in 1..=steps as usize {
                let t = step as f64 / steps;
                route.push(along(segment, t), along(projected, t), heading);
            }
            route.segments.push(projected);
        }
        route
    }

    fn push(&mut self, lng_lat: Coord, point: Coord, heading: f64) {
        self.lng_lats.push(lng_lat);
        self.points.push(point);
        self.headings.push(heading);
    }

    /// Which of this route's samples are on `other`, ignoring stretches too short to tell.
    fn samples_on(&self, other: &SampledRoute) -> Vec<bool> {
        let mut on: Vec<bool> = self
            .points
            .iter()
            .zip(&self.headings)
            .map(|(&point, &heading)| other.passes(point, heading))
            .collect();
        debounce(&mut on);
        on
    }

    /// Whether this route runs through `point` along `heading`, in either direction.
    fn passes(&self, point: Coord, heading: f64) -> bool {
        let max_heading_difference = MAX_HEADING_DIFFERENCE_DEGREES.to_radians();
        self.segments.iter().any(|segment| {
            Euclidean.distance(segment, &Point::from(point)) <= TOLERANCE_METERS
                && axis_difference(heading, segment.dy().atan2(segment.dx()))
                    <= max_heading_difference
        })
    }

    /// The stretches of this route between consecutive samples whose later one `include` accepts.
    fn stretches(&self, include: impl Fn(usize) -> bool) -> Vec<LineString> {
        let mut stretches = vec![];
        let mut current: Option<Vec<Coord>> = None;
        for sample in 1..self.lng_lats.len() {
            if include(sample) {
                current
                    .get_or_insert_with(|| vec![self.lng_lats[sample - 1]])
                    .push(self.lng_lats[sample]);
            } else if let Some(stretch) = current.take() {
                stretches.push(stretch);
            }
        }
        stretches.extend(current);
        stretches
            .into_iter()
            .map(|stretch| LineString::new(stretch).simplify(SIMPLIFY_EPSILON_DEGREES))
            .collect()
    }
}

fn along(line: Line, t: f64) -> Coord {
    line.start + line.delta() * t
}

/// The angle between two headings, ignoring which way along it each goes.
fn axis_difference(a: f64, b: f64) -> f64 {
    let difference = (a - b).rem_euclid(PI);
    difference.min(PI - difference)
}

/// The maximal runs of equal values.
fn runs(values: &[bool]) -> Vec<Range<usize>> {
    let mut start = 0;
    values
        .chunk_by(|a, b| a == b)
        .map(|chunk| {
            start += chunk.len();
            start - chunk.len()..start
        })
        .collect()
}

/// Flips runs within the route too short to tell: first gaps, so a stray sample doesn't split a
/// shared street, then touches, like crossing another route.
fn debounce(on: &mut [bool]) {
    let min_samples = (MIN_STRETCH_METERS / SAMPLE_SPACING_METERS).round() as usize;
    for value in [false, true] {
        for run in runs(on) {
            let within = run.start > 0 && run.end < on.len();
            if on[run.start] == value && within && run.len() < min_samples {
                on[run].fill(!value);
            }
        }
    }
}

/// Whether a route leaves the one `on` was measured against, besides where their ends snapped apart.
fn strays(on: &[bool]) -> bool {
    let max_end_snap = (MAX_END_SNAP_METERS / SAMPLE_SPACING_METERS) as usize;
    runs(on).into_iter().any(|run| {
        let at_end = run.start == 0 || run.end == on.len();
        !on[run.start] && (!at_end || run.len() > max_end_snap)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Meters east and north of a point in Seattle.
    fn at(east: f64, north: f64) -> Coord {
        let origin = coord! { x: -122.3, y: 47.6 };
        let projection = Projection::new(origin);
        coord! {
            x: origin.x + east / projection.meters_per_degree_lng,
            y: origin.y + north / METERS_PER_DEGREE_LAT,
        }
    }

    fn line(points: &[(f64, f64)]) -> LineString {
        LineString::new(
            points
                .iter()
                .map(|&(east, north)| at(east, north))
                .collect(),
        )
    }

    fn lengths(lines: &[LineString]) -> Vec<f64> {
        let projection = Projection::new(at(0.0, 0.0));
        lines
            .iter()
            .map(|line| {
                let meters: f64 = line
                    .lines()
                    .map(|segment| {
                        let delta =
                            projection.project(segment.end) - projection.project(segment.start);
                        delta.x.hypot(delta.y)
                    })
                    .sum();
                meters.round()
            })
            .collect()
    }

    /// Matches are approximate, so where a stretch starts and ends is too.
    fn assert_lengths(lines: &[LineString], expected: &[f64]) {
        let actual = lengths(lines);
        let close = actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| (a - e).abs() <= 8.0);
        assert!(close, "expected lengths near {expected:?}, got {actual:?}");
    }

    fn itineraries(comparison: &Comparison) -> Vec<Vec<usize>> {
        comparison
            .routes
            .iter()
            .map(|route| route.itineraries.clone())
            .collect()
    }

    #[test]
    fn routers_on_the_same_streets_share_a_route() {
        let street = line(&[(0.0, 0.0), (500.0, 0.0), (500.0, 500.0)]);
        // Placed through other vertices, and snapped to the trip's start 15m away.
        let same_streets = line(&[(15.0, 0.4), (250.0, -0.5), (500.4, 0.0), (500.0, 500.0)]);
        let comparison = Comparison::of(&[("otp", street), ("valhalla", same_streets)]);
        assert_eq!(itineraries(&comparison), [vec![0, 1]]);
        assert_lengths(&comparison.common, &[1000.0]);
    }

    #[test]
    fn one_router_planning_a_route_twice_keeps_both() {
        let street = line(&[(0.0, 0.0), (500.0, 0.0)]);
        let comparison = Comparison::of(&[("otp", street.clone()), ("otp", street)]);
        assert_eq!(itineraries(&comparison), [vec![0], vec![1]]);
    }

    #[test]
    fn a_short_detour_makes_a_different_route() {
        let road = line(&[(0.0, 0.0), (1000.0, 0.0)]);
        // Leaves the road for a bike lane 6m beside it for 40m, like around a roundabout.
        let bike_lane = line(&[
            (0.0, 0.0),
            (480.0, 0.0),
            (485.0, 6.0),
            (515.0, 6.0),
            (520.0, 0.0),
            (1000.0, 0.0),
        ]);
        let comparison = Comparison::of(&[("otp", road), ("valhalla", bike_lane)]);
        assert_eq!(itineraries(&comparison), [vec![0], vec![1]]);
        assert_eq!(comparison.common.len(), 2);
        assert_lengths(&comparison.routes[0].solo, &[40.0]);
        assert_lengths(&comparison.routes[1].solo, &[42.0]);
    }

    #[test]
    fn crossing_a_route_isnt_sharing_it() {
        let east = line(&[(0.0, 0.0), (1000.0, 0.0)]);
        let north = line(&[(500.0, -500.0), (500.0, 500.0)]);
        let comparison = Comparison::of(&[("otp", east), ("valhalla", north)]);
        assert!(comparison.common.is_empty());
        assert_lengths(&comparison.routes[0].solo, &[1000.0]);
    }

    #[test]
    fn stretches_on_some_routes_are_partly_shared() {
        let a = line(&[(0.0, 0.0), (1000.0, 0.0)]);
        let b = line(&[(0.0, 0.0), (500.0, 0.0), (500.0, 500.0)]);
        let c = line(&[(500.0, -500.0), (500.0, 500.0)]);
        let comparison = Comparison::of(&[("a", a), ("b", b), ("c", c)]);
        assert!(comparison.common.is_empty());
        assert_lengths(&comparison.routes[0].partly_shared, &[500.0]);
        assert_lengths(&comparison.routes[0].solo, &[500.0]);
        assert_lengths(&comparison.routes[1].partly_shared, &[1000.0]);
        assert!(comparison.routes[1].solo.is_empty());
        assert_lengths(&comparison.routes[2].solo, &[500.0]);
        assert_lengths(&comparison.routes[2].partly_shared, &[500.0]);
    }

    #[test]
    fn a_route_alone_is_all_common() {
        let comparison = Comparison::of(&[("otp", line(&[(0.0, 0.0), (100.0, 0.0)]))]);
        assert_lengths(&comparison.common, &[100.0]);
        assert!(comparison.routes[0].solo.is_empty());
    }
}
