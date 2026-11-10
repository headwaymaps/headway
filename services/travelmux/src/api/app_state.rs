use crate::{otp::PreparedOtpCluster, valhalla::ValhallaRouter, Error, Result};
use url::Url;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OtpCluster`].
#[derive(Debug, Clone)]
pub struct AppState {
    /// By slug, the first planning trips that don't name one.
    otp_clusters: Vec<(String, PreparedOtpCluster)>,
    /// By slug, the first planning trips that don't name one.
    valhalla_routers: Vec<(String, ValhallaRouter)>,
}

/// A router a request named by its slug.
pub enum NamedRouter<'a> {
    Otp(&'a PreparedOtpCluster),
    Valhalla(&'a ValhallaRouter),
}

impl AppState {
    /// Without any `valhalla_endpoints`, OTP plans every trip.
    pub fn new(
        otp_clusters: Vec<(String, PreparedOtpCluster)>,
        valhalla_endpoints: Vec<(String, Url)>,
    ) -> Self {
        log::debug!("new AppState with valhalla_endpoints: {valhalla_endpoints:?}");
        assert!(!otp_clusters.is_empty(), "an AppState needs an OTP cluster");
        let valhalla_routers = valhalla_endpoints
            .into_iter()
            .map(|(slug, url)| (slug, ValhallaRouter::new(url)))
            .collect();
        Self {
            otp_clusters,
            valhalla_routers,
        }
    }

    pub fn otp_cluster(&self) -> &PreparedOtpCluster {
        &self.otp_clusters[0].1
    }

    pub fn has_valhalla(&self) -> bool {
        !self.valhalla_routers.is_empty()
    }

    pub fn valhalla_router(&self) -> Result<&ValhallaRouter> {
        self.valhalla_routers
            .first()
            .map(|(_, router)| router)
            .ok_or_else(|| Error::server("no valhalla endpoint is configured"))
    }

    pub fn router(&self, slug: &str) -> Result<NamedRouter<'_>> {
        let otp = self.otp_clusters.iter().find(|(s, _)| s == slug);
        let valhalla = self.valhalla_routers.iter().find(|(s, _)| s == slug);
        match (otp, valhalla) {
            (Some((_, cluster)), _) => Ok(NamedRouter::Otp(cluster)),
            (_, Some((_, router))) => Ok(NamedRouter::Valhalla(router)),
            _ => Err(Error::user(format!("no router is called {slug}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_state() -> AppState {
        AppState::new(
            vec![("otp-headway".to_string(), PreparedOtpCluster::default())],
            ["valhalla-headway", "valhalla-master"]
                .map(|slug| {
                    let url = format!("http://{slug}:8002").parse().unwrap();
                    (slug.to_string(), url)
                })
                .to_vec(),
        )
    }

    #[test]
    fn requests_name_any_router_by_slug() {
        let app_state = app_state();
        assert!(matches!(
            app_state.router("otp-headway"),
            Ok(NamedRouter::Otp(_))
        ));
        let Ok(NamedRouter::Valhalla(router)) = app_state.router("valhalla-master") else {
            panic!("expected valhalla-master");
        };
        assert_eq!(router.endpoint().host_str(), Some("valhalla-master"));
        assert!(app_state.router("osrm").is_err());
    }

    #[test]
    fn the_first_valhalla_plans_requests_that_name_none() {
        let router = app_state().valhalla_router().unwrap().endpoint().clone();
        assert_eq!(router.host_str(), Some("valhalla-headway"));
    }
}
