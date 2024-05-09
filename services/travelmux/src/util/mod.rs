pub mod format;

use geo::{Point, Rect};
use serde::{
    ser::{Error, SerializeStruct},
    Deserialize, Deserializer, Serializer,
};
use std::time::{Duration, SystemTime};

pub fn deserialize_point_from_lat_lon<'de, D>(deserializer: D) -> Result<Point, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let s: String = Deserialize::deserialize(deserializer)?;
    use std::str::FromStr;
    let mut iter = s.split(',').map(f64::from_str);

    let Some(lat_res) = iter.next() else {
        return Err(D::Error::custom("missing lat"));
    };
    let lat = lat_res.map_err(|e| D::Error::custom(format!("invalid lat: {e}")))?;

    let Some(lon_res) = iter.next() else {
        return Err(D::Error::custom("missing lon"));
    };
    let lon = lon_res.map_err(|e| D::Error::custom(format!("invalid lon: {e}")))?;

    if let Some(next) = iter.next() {
        return Err(D::Error::custom(format!(
            "found an extra param in lat,lon,???: {next:?}"
        )));
    }

    Ok(Point::new(lon, lat))
}

pub fn deserialize_duration_from_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let seconds = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(seconds))
}

pub fn serialize_duration_as_seconds<S>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

pub fn extend_bounds(bounds: &mut Rect, extension: &Rect) {
    let min_x = f64::min(bounds.min().x, extension.min().x);
    let min_y = f64::min(bounds.min().y, extension.min().y);
    let max_x = f64::max(bounds.max().x, extension.max().x);
    let max_y = f64::max(bounds.max().y, extension.max().y);
    let mut new_bounds = Rect::new(
        geo::coord! { x: min_x, y: min_y},
        geo::coord! { x: max_x, y: max_y },
    );
    std::mem::swap(bounds, &mut new_bounds);
}

pub fn serialize_rect_to_lng_lat<S: Serializer>(
    rect: &Rect,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut struct_serializer = serializer.serialize_struct("BBox", 2)?;
    struct_serializer.serialize_field("min", &[rect.min().x, rect.min().y])?;
    struct_serializer.serialize_field("max", &[rect.max().x, rect.max().y])?;
    struct_serializer.end()
}

pub fn serialize_system_time_as_millis<S>(
    time: &SystemTime,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let since_epoch = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_e| S::Error::custom("time is before epoch"))?;
    serializer.serialize_u64(since_epoch.as_millis() as u64)
}

pub fn system_time_from_millis(millis: u64) -> SystemTime {
    std::time::UNIX_EPOCH + Duration::from_millis(millis)
}
