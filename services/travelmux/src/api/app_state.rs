use crate::{otp::PreparedOtpCluster, valhalla::ValhallaRouter};
use url::Url;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OtpCluster`].
#[derive(Debug, Clone)]
pub struct AppState {
    otp_cluster: PreparedOtpCluster,
    valhalla_router: ValhallaRouter,
}

impl AppState {
    pub fn new(valhalla_endpoint: Url, otp_cluster: PreparedOtpCluster) -> Self {
        log::debug!("new AppState with valhalla_endpoint: {valhalla_endpoint:?}");
        let valhalla_router = ValhallaRouter::new(valhalla_endpoint);
        Self {
            valhalla_router,
            otp_cluster,
        }
    }

    pub fn otp_cluster(&self) -> &PreparedOtpCluster {
        &self.otp_cluster
    }

    pub fn valhalla_router(&self) -> &ValhallaRouter {
        &self.valhalla_router
    }
}
