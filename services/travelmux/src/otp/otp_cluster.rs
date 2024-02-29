use crate::otp::otp_router::{OTPRouter, OTPRouterClient};
use crate::Result;
use geo::geometry::Point;
use url::Url;
use wkt::ToWkt;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OtpCluster {
    routers: Vec<OTPRouter>,
}

impl OtpCluster {
    pub async fn insert_endpoint(&mut self, url: Url) -> Result<()> {
        for router in OTPRouterClient::new(url).fetch_all().await? {
            self.push_router(router);
        }
        Ok(())
    }

    pub fn push_router(&mut self, router: OTPRouter) {
        self.routers.push(router)
    }

    pub fn find_router_url(&self, source: Point, destination: Point) -> Option<Url> {
        let router = self.find_router(source, destination)?;
        let router_url = OTPRouterClient::router_url(router);
        Some(router_url)
    }

    fn find_router(&self, source: Point, destination: Point) -> Option<&OTPRouter> {
        for router in &self.routers {
            use geo::algorithm::Contains;
            if !router.polygon().contains(&source) {
                log::debug!(
                    "trip source isn't within router: ({} NOT WITHIN {})",
                    source.wkt_string(),
                    router.polygon().wkt_string()
                );
                continue;
            }
            if !router.polygon().contains(&destination) {
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

    pub fn router_len(&self) -> usize {
        self.routers.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use geo::geometry::Polygon;

    #[test]
    fn no_router() {
        let cluster = OtpCluster::default();
        let from = Point::new(0.0, 0.0);
        let to = Point::new(0.0, 0.0);
        let result = cluster.find_router_url(from, to);
        assert_eq!(result, None);
    }

    #[test]
    fn found_router() {
        use wkt::TryFromWkt;

        let mut cluster = OtpCluster::default();

        {
            let endpoint_1 = Url::parse("http://host_1.example.com/foo").unwrap();
            let polygon_1 =
                Polygon::try_from_wkt_str("POLYGON ((0 0, 40 0, 40 40, 0 40, 0 0))").unwrap();
            let router_1 = OTPRouter::new(endpoint_1, "router_1".to_string(), polygon_1);
            cluster.push_router(router_1);
        }
        // points in polygon_1
        let p1_a = Point::new(1.0, 1.0);
        let p1_b = Point::new(2.0, 2.0);

        {
            let endpoint_2 = Url::parse("http://host_2.example.com/foo").unwrap();
            let polygon_2 = Polygon::try_from_wkt_str(
                "POLYGON ((100 100, 140 100, 140 140, 100 140, 100 100))",
            )
            .unwrap();
            let router_2 = OTPRouter::new(endpoint_2, "router_2".to_string(), polygon_2);
            cluster.push_router(router_2);
        }
        // points in polygon_2
        let p2_a = Point::new(101.0, 101.0);
        let p2_b = Point::new(102.0, 102.0);

        // points in neither polygon
        let p3_a = Point::new(-1.0, -1.0);
        let p3_b = Point::new(-2.0, -2.0);

        {
            let result = cluster
                .find_router_url(p1_a, p1_b)
                .expect("should have found a result");
            let expected = Url::parse("http://host_1.example.com/foo/router_1/plan").unwrap();
            assert_eq!(result, expected);
        }

        {
            let result = cluster
                .find_router_url(p2_a, p2_b)
                .expect("should have found a result");
            let expected = Url::parse("http://host_2.example.com/foo/router_2/plan").unwrap();
            assert_eq!(result, expected);
        }

        // neither point covered by a router
        {
            let result = cluster.find_router_url(p3_a, p3_b);
            assert_eq!(result, None);
        }

        // one point covered by a router, one point not covered by any router
        {
            let result = cluster.find_router_url(p1_a, p3_b);
            assert_eq!(result, None);
        }

        // both points covered by different routers
        {
            let result = cluster.find_router_url(p1_a, p2_b);
            assert_eq!(result, None);
        }
    }
}
