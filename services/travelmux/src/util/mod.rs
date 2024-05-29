pub mod format;
pub mod haversine_segmenter;
pub(crate) mod serde_util;

use crate::DistanceUnit;
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
