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

/// A mode to route by, and how to weigh its routes.
#[derive(Debug, Clone, PartialEq)]
pub enum Costing {
    Auto,
    Bicycle(BicycleCostingOptions),
    Pedestrian,
}

impl Costing {
    pub fn mode(&self) -> ModeCosting {
        match self {
            Self::Auto => ModeCosting::Auto,
            Self::Bicycle(_) => ModeCosting::Bicycle,
            Self::Pedestrian => ModeCosting::Pedestrian,
        }
    }
}

/// Bikes are routed the comfier way.
impl From<ModeCosting> for Costing {
    fn from(mode: ModeCosting) -> Self {
        match mode {
            ModeCosting::Auto => Self::Auto,
            ModeCosting::Bicycle => Self::Bicycle(BicycleCostingOptions::COMFIER),
            ModeCosting::Pedestrian => Self::Pedestrian,
        }
    }
}

/// Parameters for a query, as in:
///     `route?json={%22locations%22:[{%22lat%22:47.575837,%22lon%22:-122.339414},{%22lat%22:47.651048,%22lon%22:-122.347234}],%22costing%22:%22auto%22,%22alternates%22:3,%22units%22:%22miles%22}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValhallaRouteQuery {
    pub locations: Vec<RouteLocation>,
    pub costing: ModeCosting,
    pub alternates: u32,
    pub units: DistanceUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costing_options: Option<CostingOptions>,
}

/// Overrides of Valhalla's costing defaults, keyed by costing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostingOptions {
    pub bicycle: BicycleCostingOptions,
}

/// Bicycle costing knobs, some of which only our Valhalla fork has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BicycleCostingOptions {
    /// How much each road class, motorway through service/other, stresses a rider.
    pub road_class_factors: [f64; 8],
    /// Scales the stress of a road on a bike network, except its road class's.
    pub bicycle_network_factor: f64,
    pub use_roads: f64,
    pub use_hills: f64,
    pub avoid_bad_surfaces: f64,
    pub walk_bike_factor: f64,
    pub destination_only_penalty: f64,
    pub gate_penalty: f64,
}

impl BicycleCostingOptions {
    /// Quiet streets, paths and bike routes. Tuned against a corpus of Seattle trips for the mix
    /// of roads that best matched OTP's.
    pub const COMFIER: Self = Self {
        road_class_factors: [20.0, 8.0, 1.25, 1.0, 0.5, 0.25, 0.25, 1.0],
        bicycle_network_factor: 0.275,
        use_roads: 0.1,
        use_hills: 0.25,
        avoid_bad_surfaces: 0.5,
        walk_bike_factor: 3.0,
        destination_only_penalty: 300.0,
        gate_penalty: 150.0,
    };

    /// As little time as it takes, which already reflects hills, rough surfaces and walking
    /// the bike, so nothing else is penalized. Valhalla's own road class factors keep a mild
    /// aversion to the biggest roads for 0.1% more time.
    ///
    /// Across the Seattle corpus this takes 12.7% less time than [`Self::COMFIER`].
    pub const FASTER: Self = Self {
        road_class_factors: [1.0, 0.4, 0.2, 0.1, 0.05, 0.05, 0.0, 0.5],
        bicycle_network_factor: 1.0,
        use_roads: 1.0,
        use_hills: 1.0,
        avoid_bad_surfaces: 0.0,
        walk_bike_factor: 1.0,
        destination_only_penalty: 300.0,
        gate_penalty: 150.0,
    };
}

/// A place to route from, through, or to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteLocation {
    #[serde(flatten)]
    pub lon_lat: LonLat,
    pub r#type: LocationType,
    /// Absent to start or end on the road nearest the place.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub snap: Option<SnapToAnyRoad>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocationType {
    /// Where a trip starts or ends.
    Break,
    /// A place the trip passes on its way, without stopping.
    Through,
}

/// Lets a route start or end on any road within `radius` meters, rather than only the nearest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapToAnyRoad {
    pub radius: u32,
    /// False to choose among the roads within `radius` by route cost alone, rather than
    /// favoring the nearest.
    pub rank_candidates: bool,
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
    pub locations: Vec<TripLocation>,
    pub summary: Summary,
    pub units: DistanceUnit, // legs: Vec<Leg>
    pub legs: Vec<Leg>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Trip {
    /// Where each leg starts or ends, leaving out the places the trip only passes through.
    pub fn stops(&self) -> Vec<LonLat> {
        self.locations
            .iter()
            .filter(|location| location.r#type == LocationType::Break)
            .map(|location| location.lon_lat)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TripLocation {
    #[serde(flatten)]
    pub lon_lat: LonLat,
    pub r#type: LocationType,
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
