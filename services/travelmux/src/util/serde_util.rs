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
    let mut iter = s.split(',').map(str::trim).map(f64::from_str);

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

/// `<lon>,<lat>`, for a point carried in a query string.
pub fn deserialize_point_from_lon_lat<'de, D>(deserializer: D) -> Result<Point, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let s: String = Deserialize::deserialize(deserializer)?;
    use std::str::FromStr;
    let mut iter = s.split(',').map(str::trim).map(f64::from_str);

    let Some(lon_res) = iter.next() else {
        return Err(D::Error::custom("missing lon"));
    };
    let lon = lon_res.map_err(|e| D::Error::custom(format!("invalid lon: {e}")))?;

    let Some(lat_res) = iter.next() else {
        return Err(D::Error::custom("missing lat"));
    };
    let lat = lat_res.map_err(|e| D::Error::custom(format!("invalid lat: {e}")))?;

    if let Some(next) = iter.next() {
        return Err(D::Error::custom(format!(
            "found an extra param in lon,lat,???: {next:?}"
        )));
    }

    // A client that sent v7's `lat,lon` lands here whenever its longitude is outside a
    // latitude's range, which is most of the populated world. Worth a clear error rather than
    // planning a trip somewhere the caller didn't ask about.
    if !(-90.0..=90.0).contains(&lat) {
        return Err(D::Error::custom(format!(
            "latitude {lat} is out of range - v8 writes points as lon,lat"
        )));
    }

    Ok(Point::new(lon, lat))
}

/// v8 writes every point as a `[lon, lat]` pair, so this is how they come back in.
pub fn deserialize_point_from_lon_lat_pair<'de, D>(deserializer: D) -> Result<Point, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Point::from(<[f64; 2]>::deserialize(deserializer)?))
}

pub fn deserialize_optional_point_from_lon_lat_pair<'de, D>(
    deserializer: D,
) -> Result<Option<Point>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<[f64; 2]>::deserialize(deserializer)?.map(Point::from))
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

/// v8 writes point collections as `[lon, lat]` pairs too.
pub fn serialize_points_as_lon_lat_pairs<S>(
    points: &[Point],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_seq(points.iter().map(|point| [point.x(), point.y()]))
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
