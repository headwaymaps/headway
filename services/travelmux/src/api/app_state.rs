use crate::elevation::ElevationService;
use crate::{otp::PreparedOtpCluster, valhalla::ValhallaRouter};
use std::path::PathBuf;
use url::Url;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OtpCluster`].
#[derive(Debug, Clone)]
pub struct AppState {
    otp_cluster: PreparedOtpCluster,
    valhalla_router: ValhallaRouter,
    elevation: ElevationService,
}

impl AppState {
    pub fn new(valhalla_endpoint: Url, tif_dir: PathBuf, otp_cluster: PreparedOtpCluster) -> Self {
        log::debug!("new AppState with valhalla_endpoint: {valhalla_endpoint:?}");
        let valhalla_router = ValhallaRouter::new(valhalla_endpoint);
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

    pub fn valhalla_router(&self) -> &ValhallaRouter {
        &self.valhalla_router
    }

    pub fn elevation(&self) -> &ElevationService {
        &self.elevation
    }
}
