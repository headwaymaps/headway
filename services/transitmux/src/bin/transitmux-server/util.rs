use geo::Point;
use serde::{Deserialize, Deserializer};

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
