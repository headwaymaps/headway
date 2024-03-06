pub mod otp_api;

mod otp_cluster;
use crate::TravelMode;
pub use otp_cluster::OtpCluster;

mod otp_router;

impl From<otp_api::TransitMode> for TravelMode {
    fn from(mode: otp_api::TransitMode) -> Self {
        match mode {
            otp_api::TransitMode::Walk => TravelMode::Walk,
            otp_api::TransitMode::Bicycle => TravelMode::Bicycle,
            otp_api::TransitMode::Car => TravelMode::Car,
            otp_api::TransitMode::Tram => TravelMode::Transit,
            otp_api::TransitMode::Subway => TravelMode::Transit,
            otp_api::TransitMode::Rail => TravelMode::Transit,
            otp_api::TransitMode::Bus => TravelMode::Transit,
            otp_api::TransitMode::Ferry => TravelMode::Transit,
            otp_api::TransitMode::CableCar => TravelMode::Transit,
            otp_api::TransitMode::Gondola => TravelMode::Transit,
            otp_api::TransitMode::Funicular => TravelMode::Transit,
            otp_api::TransitMode::Transit => TravelMode::Transit,
        }
    }
}
