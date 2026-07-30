//! The v7 API.
//!
//! Where v6 was shaped by OTP's old REST API - unix-millis timestamps, distances in whichever
//! units the client asked for, and the raw upstream response echoed back under `_otp` - v7 says
//! things the way OTP's GraphQL API does:
//!
//! - times are RFC 3339 timestamps, in the timezone of the graph that planned the trip
//! - distances are always meters, durations always seconds, and the field names say so
//! - transit legs are our own type rather than a passthrough of OTP's, so there's no `_otp` to
//!   fall back on for anything
pub mod directions;
pub mod elevation;
mod error;
mod osrm_api;
pub mod plan;
mod travel_modes;

pub use error::{PlanResponseErr, PlanResponseOk};
pub use plan::Itinerary;
pub use travel_modes::TravelModes;
