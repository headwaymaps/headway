//! Types describing the shape of an OTP trip plan.
//!
//! Historically these types were deserialized directly from OTP's now-removed REST `/plan`
//! endpoint. As of OTP 2.8 the REST API is gone, so we now talk to OTP over the GTFS GraphQL
//! API (see [`crate::otp::gtfs_graphql`]) and *map* the GraphQL response into these structs.
//!
//! Their `Serialize` shape is still part of travelmux's public contract: the raw plan is echoed
//! back to clients under the `_otp` key, and transit legs are passed through verbatim. Keep the
//! serialized field names stable unless you're intentionally changing that contract.

use serde::{Deserialize, Serialize};

// The direction enums below are shared with the GraphQL layer: `cynic::Enum` derives the
// `Serialize`/`Deserialize` impls that carry the SCREAMING_SNAKE_CASE names OTP uses, which is
// also the spelling our own clients expect, and checks the variants against OTP's schema.
use crate::otp::schema;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanError {
    pub id: u32,
    // Readable English message text
    pub msg: String,
    // a stable message key
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanResponse {
    pub plan: Plan,

    // Note that `plan` will be present even if error is present, but plan.itineraries will be []
    pub error: Option<PlanError>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub itineraries: Vec<Itinerary>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Itinerary {
    /// seconds
    pub duration: u64,
    pub legs: Vec<Leg>,
    /// unix mills, UTC
    pub start_time: u64,
    /// unix mills, UTC
    pub end_time: u64,
    /// meters walked across the whole itinerary
    pub walk_distance: f64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Leg {
    pub mode: TransitMode,
    pub distance: f64,
    pub leg_geometry: LegGeometry,

    /// Whether this leg is a transit leg (as opposed to a walk/bike/car access leg).
    pub transit_leg: bool,

    // The following transit-only fields were flattened onto the leg by the old REST API.
    // We reconstruct them here from the nested GraphQL `route`/`agency` so the serialized shape
    // stays stable for clients (the web frontend reads these directly off the `_otp` legs).
    /// Route short name (empty/omitted for non-transit legs, matching the old REST behaviour).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_short_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_long_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agency_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headsign: Option<String>,

    #[serde(default)]
    pub alerts: Vec<Alert>,

    // Present, but empty, for transit legs. Non-empty for non-transit legs.
    #[serde(default)]
    pub steps: Vec<Step>,

    pub from: Place,
    pub to: Place,

    /// What time the leg starts, in millis since Unix epoch (UTC)
    pub start_time: u64,

    /// What time the leg ends, in millis since Unix epoch (UTC)
    pub end_time: u64,

    /// Whether there is real-time data about this Leg
    pub real_time: bool,
}

impl Leg {
    pub(crate) fn duration_seconds(&self) -> f64 {
        (self.end_time - self.start_time) as f64 / 1000.0
    }
}

/// A service alert (e.g. detour, elevator outage) affecting a transit leg.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_header_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_description_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_url: Option<String>,
    /// millis since Unix epoch, or None if unknown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_start_date: Option<i64>,
    /// millis since Unix epoch, or None if unknown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_end_date: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// The distance in meters that this step takes.
    pub distance: f64,
    /// The relative direction of this step.
    pub relative_direction: RelativeDirection,
    /// The name of the street.
    pub street_name: String,
    /// The absolute (compass) direction of this step.
    pub absolute_direction: Option<AbsoluteDirection>,
    /// When exiting a highway or traffic circle, the exit name/number.
    pub exit: Option<String>,
    /// Indicates whether or not a street changes direction at an intersection.
    pub stay_on: Option<bool>,
    /// This step is on an open area, such as a plaza or train platform, and thus the directions should say something like "cross"
    pub area: Option<bool>,
    /// The name of this street was generated by the system, so we should only display it once, and generally just display right/left directions
    pub bogus_name: Option<bool>,
    /// The longitude of start of the step
    pub lon: f64,
    /// The latitude of start of the step
    pub lat: f64,
}

#[derive(Debug, PartialEq, Clone, cynic::Enum)]
pub enum AbsoluteDirection {
    North,
    Northeast,
    East,
    Southeast,
    South,
    Southwest,
    West,
    Northwest,
}

#[derive(Debug, PartialEq, Clone, Copy, cynic::Enum)]
pub enum RelativeDirection {
    Depart,
    HardLeft,
    Left,
    SlightlyLeft,
    Continue,
    SlightlyRight,
    Right,
    HardRight,
    CircleClockwise,
    CircleCounterclockwise,
    Elevator,
    UturnLeft,
    UturnRight,
    // Newer OTP GraphQL directions (station entrances / signage). We don't produce turn-by-turn
    // instructions for transit legs, so these just need to round-trip.
    EnterStation,
    ExitStation,
    FollowSigns,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct LonLat {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    #[serde(flatten)]
    pub location: LonLat,

    /// millis since Unix epoch
    /// I think it's None iff it's the trip Origin
    pub arrival: Option<u64>,

    /// millis since Unix epoch
    /// I think it's None iff it's the trip Destination
    pub departure: Option<u64>,

    /// "Civic Center / Grand Park Station"
    /// Transit stops often have names. But this is often blank when
    /// the place is some random lat/lon (e.g. the users destination)
    pub name: Option<String>,
}

use crate::valhalla::valhalla_api::ManeuverType as ValhallaManeuverType;
impl From<RelativeDirection> for ValhallaManeuverType {
    fn from(otp: RelativeDirection) -> Self {
        match otp {
            RelativeDirection::Depart => ValhallaManeuverType::Start,
            RelativeDirection::HardLeft => ValhallaManeuverType::SharpLeft,
            RelativeDirection::Left => ValhallaManeuverType::Left,
            RelativeDirection::SlightlyLeft => ValhallaManeuverType::SlightLeft,
            RelativeDirection::Continue => ValhallaManeuverType::Continue,
            RelativeDirection::SlightlyRight => ValhallaManeuverType::SlightRight,
            RelativeDirection::Right => ValhallaManeuverType::Right,
            RelativeDirection::HardRight => ValhallaManeuverType::SharpRight,
            RelativeDirection::CircleClockwise => ValhallaManeuverType::RoundaboutEnter,
            RelativeDirection::CircleCounterclockwise => ValhallaManeuverType::RoundaboutEnter,
            RelativeDirection::Elevator => ValhallaManeuverType::ElevatorEnter,
            RelativeDirection::UturnLeft => ValhallaManeuverType::UturnLeft,
            RelativeDirection::UturnRight => ValhallaManeuverType::UturnRight,
            // These only occur on transit legs, where we don't emit turn-by-turn maneuvers, but
            // map them to something sensible for completeness.
            RelativeDirection::EnterStation
            | RelativeDirection::ExitStation
            | RelativeDirection::FollowSigns => ValhallaManeuverType::Continue,
        }
    }
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
