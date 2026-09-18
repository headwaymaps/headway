//! The v8 API.
//!
//! v7's shape, with every point written the same way: a `[lon, lat]` pair, in requests and
//! responses alike. v7 wrote them three different ways - `lat,lon` query strings, `lat`/`lon`
//! object fields, and `[lon, lat]` pairs in the OSRM-shaped routes - and a client had to know
//! which one each endpoint meant.
//!
//! Carried over from v7:
//!
//! - times are RFC 3339 timestamps, in the timezone of the graph that planned the trip
//! - distances are always meters, durations always seconds, and the field names say so
//! - transit legs are our own type rather than a passthrough of OTP's, so there's no `_otp` to
//!   fall back on for anything
//!
//! Live vehicle positions are v8-only; v7 never had them.
pub mod directions;
pub mod elevation;
mod error;
mod osrm_api;
pub mod plan;
mod travel_modes;
pub mod vehicle_positions;

pub use error::{PlanResponseErr, PlanResponseOk};
pub use plan::Itinerary;
pub use travel_modes::TravelModes;
