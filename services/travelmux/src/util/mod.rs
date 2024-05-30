pub mod format;
pub mod haversine_segmenter;
pub(crate) mod serde_util;

use crate::DistanceUnit;
use geo::{Coord, LineString, Point};
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
pub(crate) fn bearing_between(start: Option<Coord>, end: Option<Coord>) -> Option<u16> {
    let start = Point(start?);
    let end = Point(end?);
    use geo::HaversineBearing;
    let bearing = start.haversine_bearing(end);
    debug_assert!(bearing >= -180.0);
    Some((bearing.round() + 360.0) as u16 % 360)
}

pub(crate) fn bearing_at_start(line_string: &LineString) -> Option<u16> {
    let first = line_string.0.first();
    let second = line_string.0.get(1);
    bearing_between(first.copied(), second.copied())
}

pub(crate) fn bearing_at_end(line_string: &LineString) -> Option<u16> {
    let last = line_string.0.last();
    let second_to_last = line_string.0.get(line_string.0.len() - 2);
    bearing_between(second_to_last.copied(), last.copied())
}
