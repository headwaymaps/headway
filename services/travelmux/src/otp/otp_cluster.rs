use crate::otp::otp_router::{OTPRouter, OTPRouterClient, PreparedOTPRouter};
use crate::{Error, Result};
use geo::geometry::{Point, Polygon};
use geo::PreparedGeometry;
use url::Url;
use wkt::ToWkt;

#[derive(Debug, Clone)]
pub struct OtpCluster<Coverage = Polygon> {
    routers: Vec<OTPRouter<Coverage>>,
}

/// An [`OtpCluster`] whose routers are ready to serve trips - see [`OTPRouter::prepare`].
pub type PreparedOtpCluster = OtpCluster<PreparedGeometry<'static, Polygon>>;

// Hand written rather than derived: the derive would needlessly require `Coverage: Default`.
impl<Coverage> Default for OtpCluster<Coverage> {
    fn default() -> Self {
        Self { routers: vec![] }
    }
}

impl<Coverage> OtpCluster<Coverage> {
    pub fn router_len(&self) -> usize {
        self.routers.len()
    }
}

impl OtpCluster {
    /// `base_url` is the root of an OTP instance, e.g. `http://opentripplanner:8000` — the
    /// GraphQL API's path is appended by [`OTPRouterClient`].
    pub async fn insert_endpoint(&mut self, base_url: &str) -> Result<()> {
        log::info!("adding endpoint: {base_url}");
        let url = Url::parse(base_url).map_err(|err| {
            log::error!("error while parsing endpoint url {base_url:?}");
            Error::server(format!("invalid endpoint url: {err}"))
        })?;

        // TODO: Separate inserting an endpoint from (periodically) fetching its routers
        let routers = OTPRouterClient::new(url)?
            .fetch_all()
            .await
            .inspect_err(|err| {
                log::error!("error while inserting endpoint {base_url:?}, {err}");
            })?;

        for router in routers {
            self.push_router(router);
        }
        log::info!("added endpoint: {base_url}");
        Ok(())
    }

    pub fn push_router(&mut self, router: OTPRouter) {
        self.routers.push(router)
    }

    /// Index every router's coverage area for serving requests. See [`OTPRouter::prepare`].
    pub fn prepare(&self) -> PreparedOtpCluster {
        OtpCluster {
            routers: self.routers.iter().map(OTPRouter::prepare).collect(),
        }
    }
}

impl PreparedOtpCluster {
    /// Find the OTP instance whose coverage area contains both `source` and `destination`.
    pub fn find_router(&self, source: Point, destination: Point) -> Option<&PreparedOTPRouter> {
        for router in &self.routers {
            if !router.contains(&source) {
                log::debug!(
                    "trip source isn't within router: ({} NOT WITHIN {})",
                    source.wkt_string(),
                    router.polygon().wkt_string()
                );
                continue;
            }
            if !router.contains(&destination) {
                log::debug!(
                    "trip destination isn't within router: ({} NOT WITHIN {})",
                    destination.wkt_string(),
                    router.polygon().wkt_string()
                );
                continue;
            }
            return Some(router);
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use geo::wkt;

    #[test]
    fn no_router() {
        let cluster = OtpCluster::default().prepare();
        let from = Point::new(0.0, 0.0);
        let to = Point::new(0.0, 0.0);
        assert!(cluster.find_router(from, to).is_none());
    }

    #[test]
    fn found_router() {
        let mut cluster = OtpCluster::default();

        let endpoint_1 = Url::parse("http://host_1.example.com/otp/gtfs/v1").unwrap();
        {
            let polygon_1 = wkt! { POLYGON((0.0 0.0, 40.0 0.0, 40.0 40.0, 0.0 40.0, 0.0 0.0)) };
            let router_1 = OTPRouter::new(endpoint_1.clone(), polygon_1, None);
            cluster.push_router(router_1);
        }
        // points in polygon_1
        let p1_a = Point::new(1.0, 1.0);
        let p1_b = Point::new(2.0, 2.0);

        let endpoint_2 = Url::parse("http://host_2.example.com/otp/gtfs/v1").unwrap();
        {
            let polygon_2 = wkt! {
                POLYGON((100.0 100.0, 140.0 100.0, 140.0 140.0, 100.0 140.0, 100.0 100.0))
            };
            let router_2 = OTPRouter::new(endpoint_2.clone(), polygon_2, None);
            cluster.push_router(router_2);
        }
        // points in polygon_2
        let p2_a = Point::new(101.0, 101.0);
        let p2_b = Point::new(102.0, 102.0);

        // points in neither polygon
        let p3_a = Point::new(-1.0, -1.0);
        let p3_b = Point::new(-2.0, -2.0);

        let cluster = cluster.prepare();

        {
            let router = cluster
                .find_router(p1_a, p1_b)
                .expect("should have found a result");
            assert_eq!(router.endpoint(), &endpoint_1);
        }

        {
            let router = cluster
                .find_router(p2_a, p2_b)
                .expect("should have found a result");
            assert_eq!(router.endpoint(), &endpoint_2);
        }

        // neither point covered by a router
        assert!(cluster.find_router(p3_a, p3_b).is_none());

        // one point covered by a router, one point not covered by any router
        assert!(cluster.find_router(p1_a, p3_b).is_none());

        // both points covered by different routers
        assert!(cluster.find_router(p1_a, p2_b).is_none());
    }
}
