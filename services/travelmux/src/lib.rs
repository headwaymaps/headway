pub mod api;
mod error;
pub mod otp;
pub mod util;
pub mod valhalla;

pub use error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum TravelMode {
    Transit,
    Bicycle,
    Car,
    Walk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DistanceUnit {
    Meters,
    Kilometers,
    Miles,
}

impl DistanceUnit {
    fn measurement_system(&self) -> MeasurementSystem {
        match self {
            DistanceUnit::Meters | DistanceUnit::Kilometers => MeasurementSystem::Metric,
            DistanceUnit::Miles => MeasurementSystem::Imperial,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementSystem {
    Metric,
    Imperial,
}
