mod error;
pub mod otp;
pub mod util;
pub mod valhalla;

pub use error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TravelMode {
    Transit,
    Bicycle,
    Car,
    Walk,
}
