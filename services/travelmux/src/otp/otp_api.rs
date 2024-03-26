use geo::geometry::Polygon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Router {
    #[serde(deserialize_with = "geojson::de::deserialize_geometry")]
    pub polygon: Polygon,
    pub router_id: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Routers {
    pub router_info: Vec<Router>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanError {
    pub id: u32,
    // Readable English message text
    pub msg: String,
    // a stable message key
    pub message: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanResponse {
    pub plan: Plan,

    // Note that `plan` will be present even if error is present, but plan.itinieraries will be []
    pub error: Option<PlanError>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub itineraries: Vec<Itinerary>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Itinerary {
    pub duration: u64,
    pub legs: Vec<Leg>,
    pub end_time: u64,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub mode: TransitMode,
    pub distance: f64,
    pub leg_geometry: LegGeometry,
    pub route_color: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransitMode {
    Walk,
    Bicycle,
    Car,
    Tram,
    Subway,
    Rail,
    Bus,
    Ferry,
    CableCar,
    Gondola,
    Funicular,
    Transit,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LegGeometry {
    pub length: f64,
    pub points: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_serialization() {
        let mode = TransitMode::Walk;
        let serialized = serde_json::to_string(&mode).unwrap();
        assert_eq!(serialized, "\"WALK\"");
    }

    #[test]
    fn test_walk_deserialization() {
        let json = "\"WALK\"";
        let deserialized: TransitMode = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized, TransitMode::Walk);
    }

    #[test]
    fn test_cable_car_serialization() {
        let mode = TransitMode::CableCar;
        let serialized = serde_json::to_string(&mode).unwrap();
        assert_eq!(serialized, "\"CABLE_CAR\"");
    }

    #[test]
    fn test_cable_car_deserialization() {
        let json = "\"CABLE_CAR\"";
        let deserialized: TransitMode = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized, TransitMode::CableCar);
    }
}
