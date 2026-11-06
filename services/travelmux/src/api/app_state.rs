use crate::elevation::ElevationService;
use crate::{otp::PreparedOtpCluster, valhalla::ValhallaRouter, Error, Result};
use std::path::PathBuf;
use url::Url;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OtpCluster`].
#[derive(Debug, Clone)]
pub struct AppState {
    otp_cluster: PreparedOtpCluster,
    valhalla_router: Option<ValhallaRouter>,
    elevation: ElevationService,
}

impl AppState {
    /// Without a `valhalla_endpoint`, OTP plans every trip.
    pub fn new(
        valhalla_endpoint: Option<Url>,
        tif_dir: PathBuf,
        otp_cluster: PreparedOtpCluster,
    ) -> Self {
        log::debug!("new AppState with valhalla_endpoint: {valhalla_endpoint:?}");
        let valhalla_router = valhalla_endpoint.map(ValhallaRouter::new);
        debug_assert!(std::fs::exists(&tif_dir).unwrap());
        Self {
            valhalla_router,
            otp_cluster,
            elevation: ElevationService::new(tif_dir),
        }
    }

    pub fn otp_cluster(&self) -> &PreparedOtpCluster {
        &self.otp_cluster
    }

    pub fn has_valhalla(&self) -> bool {
        self.valhalla_router.is_some()
    }

    pub fn valhalla_router(&self) -> Result<&ValhallaRouter> {
        self.valhalla_router
            .as_ref()
            .ok_or_else(|| Error::server("no valhalla endpoint is configured"))
    }

    pub fn elevation(&self) -> &ElevationService {
        &self.elevation
    }
}
