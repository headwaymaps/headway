use crate::DistanceUnit;
use geo::Point;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub locations: Vec<LngLat>,
    pub costing: ModeCosting,
    pub alternates: u32,
    pub units: DistanceUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteResponseError {
    pub status_code: u16,
    pub error_code: u32,
    pub error: String,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteResponse {
    pub trip: Trip,
    pub alternates: Option<Vec<Alternate>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValhallaRouteResponseResult {
    Ok(RouteResponse),
    Err(RouteResponseError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trip {
    pub locations: Vec<LngLat>,
    pub summary: Summary,
    pub units: DistanceUnit, // legs: Vec<Leg>
    pub legs: Vec<Leg>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alternate {
    pub trip: Trip,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub length: f64,
    pub time: f64,
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LngLat {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Leg {
    pub summary: Summary,
    pub maneuvers: Vec<Maneuver>,
    pub shape: String,
    // pub duration: f64,
    // pub length: f64,
    // pub steps: Vec<Step>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Maneuver {
    pub instruction: String,
    pub cost: f64,
    pub begin_shape_index: u64,
    pub end_shape_index: u64,
    pub highway: Option<bool>,
    pub length: f64,
    pub street_names: Option<Vec<String>>,
    pub time: f64,
    pub travel_mode: String,
    pub travel_type: String,
    pub r#type: u64,
    pub verbal_post_transition_instruction: Option<String>,
    pub verbal_pre_transition_instruction: String,
    pub verbal_succinct_transition_instruction: Option<String>,
}

impl From<Point> for LngLat {
    fn from(value: Point) -> Self {
        Self {
            lat: value.y(),
            lon: value.x(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maneuver_from_json() {
        // deserialize a maneuver from a JSON string
        let json = r#"
        {
            "begin_shape_index": 0,
            "cost": 246.056,
            "end_shape_index": 69,
            "highway": true,
            "instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "length": 2.218,
            "street_names": [
            "Fauntleroy Way Southwest"
            ],
            "time": 198.858,
            "travel_mode": "drive",
            "travel_type": "car",
            "type": 2,
            "verbal_post_transition_instruction": "Continue for 2 miles.",
            "verbal_pre_transition_instruction": "Drive northeast on Fauntleroy Way Southwest.",
            "verbal_succinct_transition_instruction": "Drive northeast."
        }"#;

        let maneuver: Maneuver = serde_json::from_str(json).unwrap();
        assert_eq!(maneuver.r#type, 2);
        assert_eq!(
            maneuver.instruction,
            "Drive northeast on Fauntleroy Way Southwest."
        );

        assert_eq!(maneuver.highway, Some(true));
    }
}
