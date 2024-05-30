use crate::otp::otp_api;
use crate::DistanceUnit;
use geo::{Coord, Point};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
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
    pub locations: Vec<LonLat>,
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
    pub locations: Vec<LonLat>,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Leg {
    pub summary: Summary,
    pub maneuvers: Vec<Maneuver>,
    pub shape: String,
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
    /// units depend on the request
    pub length: f64,
    pub street_names: Option<Vec<String>>,
    pub time: f64,
    pub travel_mode: String,
    pub travel_type: String,
    pub r#type: ManeuverType,
    pub verbal_post_transition_instruction: Option<String>,
    // Usually, but not always, present - e.g. missing from:
    //     {
    //       "type": 39,
    //       "instruction": "Take the elevator.",
    //       "time": 21.176,
    //       "length": 0.018,
    //       "cost": 21.176,
    //       "begin_shape_index": 291,
    //       "end_shape_index": 293,
    //       "rough": true,
    //       "travel_mode": "pedestrian",
    //       "travel_type": "foot"
    //     },
    pub verbal_pre_transition_instruction: Option<String>,
    pub verbal_succinct_transition_instruction: Option<String>,
}

// Corresponding to valhalla/src/odin/maneuver.cc
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(i32)] // Using u8 assuming all values fit into an 8-bit unsigned integer
#[non_exhaustive]
pub enum ManeuverType {
    None = 0,
    Start = 1,
    StartRight = 2,
    StartLeft = 3,
    Destination = 4,
    DestinationRight = 5,
    DestinationLeft = 6,
    Becomes = 7,
    Continue = 8,
    SlightRight = 9,
    Right = 10,
    SharpRight = 11,
    UturnRight = 12,
    UturnLeft = 13,
    SharpLeft = 14,
    Left = 15,
    SlightLeft = 16,
    RampStraight = 17,
    RampRight = 18,
    RampLeft = 19,
    ExitRight = 20,
    ExitLeft = 21,
    StayStraight = 22,
    StayRight = 23,
    StayLeft = 24,
    Merge = 25,
    RoundaboutEnter = 26,
    RoundaboutExit = 27,
    FerryEnter = 28,
    FerryExit = 29,
    Transit = 30,
    TransitTransfer = 31,
    TransitRemainOn = 32,
    TransitConnectionStart = 33,
    TransitConnectionTransfer = 34,
    TransitConnectionDestination = 35,
    PostTransitConnectionDestination = 36,
    MergeRight = 37,
    MergeLeft = 38,
    ElevatorEnter = 39,
    StepsEnter = 40,
    EscalatorEnter = 41,
    BuildingEnter = 42,
    BuildingExit = 43,
}

impl From<Point> for LonLat {
    fn from(value: Point) -> Self {
        Self {
            lat: value.y(),
            lon: value.x(),
        }
    }
}

impl From<LonLat> for Point {
    fn from(value: LonLat) -> Self {
        geo::point!(x: value.lon, y: value.lat)
    }
}

impl From<LonLat> for Coord {
    fn from(value: LonLat) -> Self {
        geo::coord!(x: value.lon, y: value.lat)
    }
}

impl From<otp_api::LonLat> for LonLat {
    fn from(value: otp_api::LonLat) -> Self {
        Self {
            lon: value.lon,
            lat: value.lat,
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
        assert_eq!(maneuver.r#type, ManeuverType::StartRight);
        assert_eq!(
            maneuver.instruction,
            "Drive northeast on Fauntleroy Way Southwest."
        );

        assert_eq!(maneuver.highway, Some(true));
    }
}
