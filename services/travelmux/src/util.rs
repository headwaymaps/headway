use geo::{Point, Rect};
use serde::{Deserialize, Deserializer};
use std::time::Duration;

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
    S: serde::Serializer,
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
