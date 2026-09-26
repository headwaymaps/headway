pub mod format;
pub mod haversine_segmenter;
pub(crate) mod serde_util;

use crate::util::haversine_segmenter::HaversineSegmenter;
use crate::DistanceUnit;
use geo::{Distance, Haversine, LineString, Point};
use std::time::{Duration, SystemTime};

pub fn extend_bounds(bounds: &mut geo::Rect, extension: &geo::Rect) {
    let min_x = f64::min(bounds.min().x, extension.min().x);
    let min_y = f64::min(bounds.min().y, extension.min().y);
    let max_x = f64::max(bounds.max().x, extension.max().x);
    let max_y = f64::max(bounds.max().y, extension.max().y);
    let mut new_bounds = geo::Rect::new(
        geo::coord! { x: min_x, y: min_y},
        geo::coord! { x: max_x, y: max_y },
    );
    std::mem::swap(bounds, &mut new_bounds);
}

pub fn system_time_from_millis(millis: u64) -> SystemTime {
    std::time::UNIX_EPOCH + Duration::from_millis(millis)
}

const METERS_PER_MILE: f64 = 1609.34;

pub fn convert_from_meters(meters: f64, output_units: DistanceUnit) -> f64 {
    match output_units {
        DistanceUnit::Meters => meters,
        DistanceUnit::Kilometers => meters / 1000.0,
        DistanceUnit::Miles => meters / METERS_PER_MILE,
    }
}

pub fn convert_to_meters(distance: f64, input_units: DistanceUnit) -> f64 {
    match input_units {
        DistanceUnit::Meters => distance,
        DistanceUnit::Kilometers => distance * 1000.0,
        DistanceUnit::Miles => distance * METERS_PER_MILE,
    }
}

/// Integers from 0 to 359 representing the bearing from the last point of the current LineString
/// North is 0°, East is 90°, South is 180°, West is 270°
pub(crate) fn bearing_between(start: Point, end: Point) -> Option<u16> {
    use geo::{Bearing, Haversine};
    Some(Haversine.bearing(start, end) as u16)
}

pub(crate) fn bearing_at_start(line_string: &LineString) -> Option<u16> {
    let first = Point(*line_string.0.first()?);
    let second = Point(*line_string.0.get(1)?);
    bearing_between(first, second)
}

pub(crate) fn bearing_at_end(line_string: &LineString) -> Option<u16> {
    let last = Point(*line_string.0.last()?);
    let second_to_last = Point(*line_string.0.get(line_string.0.len() - 2)?);

    let distance = Haversine.distance(second_to_last, last);
    // Without this check, we can get some surprising bearings with OTP trips.
    //
    // OTP doesn't provide a geometry for each maneuver, only for the entire leg.
    // So we segment the leg geometry into sub-segments based on the (known) maneuver length.
    //
    // I think wha't happening is the length of the maneuver is slightly off from the actual geometry,
    // so the beginning/end of the maneuver might be just a hair past or before the actual maneuver point,
    // which is enough to through off our naive bearing calculation.
    //
    // So if the nearest coordinate is too close, we'll do this slower calculation to make sure
    // we can at least get a decent overall directional vector for the end of the maneuver.
    // This probably also applies to bearing_at_start, but I'm not currently using that field
    // for anything.
    if distance < 1.0 {
        let mut reversed_line_string = LineString(line_string.0.clone());
        reversed_line_string.0.reverse();
        let mut segmenter = HaversineSegmenter::new(reversed_line_string);
        let _discard = segmenter.next_segment(1.0)?;
        let segment = segmenter.next_segment(1.0)?;
        log::debug!("Points are too close to calculate bearing_at_end: {distance:?}");
        bearing_between(Point(*segment.0.last()?), Point(*segment.0.first()?))
    } else {
        bearing_between(second_to_last, last)
    }
}

/// The point on `shape` nearest to `point`.
pub(crate) fn closest_point_on(shape: &LineString, point: Point) -> Option<Point> {
    use geo::HaversineClosestPoint;

    match shape.haversine_closest_point(&point) {
        geo::Closest::SinglePoint(projected) | geo::Closest::Intersection(projected) => {
            Some(projected)
        }
        geo::Closest::Indeterminate => None,
    }
}

/// Haversine distance along the nearest segment of `shape`.
pub(crate) fn progress_along(shape: &LineString, point: Point) -> Option<f64> {
    use geo::{Distance, Haversine, HaversineClosestPoint};

    // (how far `point` is from the shape there, how far along the shape that is)
    let mut nearest: Option<(f64, f64)> = None;
    let mut travelled = 0.0;
    for line in shape.lines() {
        let start = Point::from(line.start);
        let projected = match line.haversine_closest_point(&point) {
            geo::Closest::SinglePoint(projected) | geo::Closest::Intersection(projected) => {
                Some(projected)
            }
            // A degenerate segment comes back as its own start, never indeterminate.
            geo::Closest::Indeterminate => None,
        };
        if let Some(projected) = projected {
            let distance = Haversine.distance(projected, point);
            if nearest.is_none_or(|(nearest, _)| distance < nearest) {
                nearest = Some((distance, travelled + Haversine.distance(start, projected)));
            }
        }
        travelled += Haversine.distance(start, Point::from(line.end));
    }

    nearest.map(|(_, progress)| progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use geo::wkt;

    /// Two blocks east, then two blocks north.
    fn corner() -> LineString {
        wkt!(LINESTRING(-122.340 47.600, -122.330 47.600, -122.330 47.610))
    }

    fn first_leg() -> f64 {
        Haversine.distance(Point::new(-122.340, 47.600), Point::new(-122.330, 47.600))
    }

    #[test]
    fn progress_accumulates_over_earlier_segments() {
        let shape = corner();

        let at_the_start = progress_along(&shape, Point::new(-122.340, 47.600)).unwrap();
        assert_relative_eq!(at_the_start, 0.0, epsilon = 1.0);

        let halfway_along = progress_along(&shape, Point::new(-122.335, 47.600)).unwrap();
        assert_relative_eq!(halfway_along, first_leg() / 2.0, epsilon = 1.0);

        let at_the_corner = progress_along(&shape, Point::new(-122.330, 47.600)).unwrap();
        assert_relative_eq!(at_the_corner, first_leg(), epsilon = 1.0);

        // Partway up the second leg: the whole first leg, plus what it has climbed.
        let up_the_second = progress_along(&shape, Point::new(-122.330, 47.605)).unwrap();
        assert!(up_the_second > first_leg());
    }

    /// A vehicle off to the side of the road still counts as being where it is alongside.
    #[test]
    fn a_point_beside_the_shape_projects_onto_it() {
        let just_north_of_the_line = Point::new(-122.335, 47.6004);
        let progress = progress_along(&corner(), just_north_of_the_line).unwrap();

        assert_relative_eq!(progress, first_leg() / 2.0, epsilon = 5.0);
    }

    #[test]
    fn a_shape_with_no_segments_has_no_progress() {
        assert_eq!(
            progress_along(&LineString::new(vec![]), Point::new(0., 0.)),
            None
        );
    }
}
