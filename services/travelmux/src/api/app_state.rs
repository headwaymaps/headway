use crate::{otp::PreparedOtpCluster, valhalla::ValhallaRouter, Error, Result};
use std::collections::HashMap;
use url::Url;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OtpCluster`].
#[derive(Debug, Clone)]
pub struct AppState {
    otp_cluster: PreparedOtpCluster,
    valhalla_router: Option<ValhallaRouter>,
    /// Other Valhallas a request can name, like a build to compare.
    named_valhalla_routers: HashMap<String, ValhallaRouter>,
}

impl AppState {
    /// Without a `valhalla_endpoint`, OTP plans every trip.
    pub fn new(
        valhalla_endpoint: Option<Url>,
        named_valhalla_endpoints: HashMap<String, Url>,
        otp_cluster: PreparedOtpCluster,
    ) -> Self {
        log::debug!("new AppState with valhalla_endpoint: {valhalla_endpoint:?}");
        let valhalla_router = valhalla_endpoint.map(ValhallaRouter::new);
        let named_valhalla_routers = named_valhalla_endpoints
            .into_iter()
            .map(|(name, url)| (name, ValhallaRouter::new(url)))
            .collect();
        Self {
            valhalla_router,
            named_valhalla_routers,
            otp_cluster,
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

    /// The Valhalla called `name`, or the default one without a name.
    pub fn named_valhalla_router(&self, name: Option<&str>) -> Result<&ValhallaRouter> {
        let Some(name) = name else {
            return self.valhalla_router();
        };
        self.named_valhalla_routers
            .get(name)
            .ok_or_else(|| Error::user(format!("no valhalla is named {name}")))
    }
}
