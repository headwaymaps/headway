use geo::{Point, Rect};
use serde::ser::{Error, SerializeStruct, SerializeTuple};
use serde::{Deserialize, Deserializer, Serializer};
use std::time::SystemTime;

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
pub fn serialize_point_as_lon_lat_pair<S>(point: &Point, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut tuple_serializer = serializer.serialize_tuple(2)?;
    tuple_serializer.serialize_element(&point.x())?;
    tuple_serializer.serialize_element(&point.y())?;
    tuple_serializer.end()
}

pub fn serialize_line_string_as_polyline6<S>(
    line_string: &geo::LineString<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let string =
        polyline::encode_coordinates(line_string.0.iter().copied(), 6).map_err(S::Error::custom)?;
    serializer.serialize_str(&string)
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
