use geo::Point;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DistanceUnit {
    Kilometers,
    Miles,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeCosting {
    Auto,
    Bicycle,
    Pedestrian,
}

/// Parameters for a query, as in:
///     `route?json={%22locations%22:[{%22lat%22:47.575837,%22lon%22:-122.339414},{%22lat%22:47.651048,%22lon%22:-122.347234}],%22costing%22:%22auto%22,%22alternates%22:3,%22units%22:%22miles%22}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValhallaRouteQuery {
    pub(crate) locations: Vec<LngLat>,
    pub(crate) costing: ModeCosting,
    pub(crate) alternates: u32,
    pub(crate) units: DistanceUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LngLat {
    lat: f64,
    lon: f64,
}

impl From<Point> for LngLat {
    fn from(value: Point) -> Self {
        Self {
            lat: value.y(),
            lon: value.x(),
        }
    }
}
