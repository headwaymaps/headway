use crate::otp::{PreparedOTPRouter, PreparedOtpCluster};
use crate::valhalla::ValhallaRouter;
use crate::{Error, Result, TravelMode};
use geo::Point;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OtpCluster`].
#[derive(Debug, Clone)]
pub struct AppState {
    /// By slug, in the order to try them for a trip that names none.
    routers: Vec<(String, Router)>,
}

#[derive(Debug, Clone)]
pub enum Router {
    Otp(PreparedOtpCluster),
    Valhalla(ValhallaRouter),
}

impl Router {
    /// Whether to offer this router a trip that doesn't name one.
    fn suits(&self, mode: TravelMode, from: Point, to: Point) -> bool {
        match self {
            Router::Otp(cluster) => {
                mode != TravelMode::Car && cluster.find_router(from, to).is_some()
            }
            Router::Valhalla(_) => mode != TravelMode::Transit,
        }
    }
}

impl AppState {
    pub fn new(routers: Vec<(String, Router)>) -> Self {
        Self { routers }
    }

    /// The routers to try, in order, for a trip that doesn't name one.
    pub fn routers_suiting(
        &self,
        mode: TravelMode,
        from: Point,
        to: Point,
    ) -> impl Iterator<Item = (&str, &Router)> {
        self.routers
            .iter()
            .filter(move |(_, router)| router.suits(mode, from, to))
            .map(|(slug, router)| (slug.as_str(), router))
    }

    pub fn router(&self, slug: &str) -> Result<&Router> {
        self.routers
            .iter()
            .find(|(s, _)| s == slug)
            .map(|(_, router)| router)
            .ok_or_else(|| Error::user(format!("no router is called {slug}")))
    }

    pub fn otp_clusters(&self) -> impl Iterator<Item = &PreparedOtpCluster> {
        self.routers.iter().filter_map(|(_, router)| match router {
            Router::Otp(cluster) => Some(cluster),
            Router::Valhalla(_) => None,
        })
    }

    /// The first OTP instance covering both ends of the trip.
    pub fn otp_router(&self, from: Point, to: Point) -> Option<&PreparedOTPRouter> {
        self.otp_clusters()
            .find_map(|cluster| cluster.find_router(from, to))
    }

    pub fn valhalla_router(&self) -> Result<&ValhallaRouter> {
        self.routers
            .iter()
            .find_map(|(_, router)| match router {
                Router::Valhalla(valhalla) => Some(valhalla),
                Router::Otp(_) => None,
            })
            .ok_or_else(|| Error::server("no valhalla endpoint is configured"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::{OTPRouter, OtpCluster};
    use geo::wkt;

    fn valhalla(slug: &str) -> (String, Router) {
        let url = format!("http://{slug}:8002").parse().unwrap();
        (slug.to_string(), Router::Valhalla(ValhallaRouter::new(url)))
    }

    fn otp(slug: &str) -> (String, Router) {
        let mut cluster = OtpCluster::default();
        cluster.push_router(OTPRouter::new(
            "Zone".to_string(),
            format!("http://{slug}:8080").parse().unwrap(),
            wkt! { POLYGON((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)) },
            None,
        ));
        (slug.to_string(), Router::Otp(cluster.prepare()))
    }

    fn app_state() -> AppState {
        AppState::new(vec![
            otp("otp-headway"),
            valhalla("valhalla-headway"),
            valhalla("valhalla-master"),
        ])
    }

    fn slugs<'a>(routers: impl Iterator<Item = (&'a str, &'a Router)>) -> Vec<&'a str> {
        routers.map(|(slug, _)| slug).collect()
    }

    #[test]
    fn requests_name_any_router_by_slug() {
        let app_state = app_state();
        assert!(matches!(
            app_state.router("otp-headway"),
            Ok(Router::Otp(_))
        ));
        let Ok(Router::Valhalla(router)) = app_state.router("valhalla-master") else {
            panic!("expected valhalla-master");
        };
        assert_eq!(router.endpoint().host_str(), Some("valhalla-master"));
        assert!(app_state.router("osrm").is_err());
    }

    #[test]
    fn unnamed_trips_try_the_routers_that_suit_them_in_order() {
        let app_state = app_state();
        let (inside, also_inside, outside) = (
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(50.0, 50.0),
        );

        let bike = app_state.routers_suiting(TravelMode::Bicycle, inside, also_inside);
        assert_eq!(
            slugs(bike),
            ["otp-headway", "valhalla-headway", "valhalla-master"]
        );

        let car = app_state.routers_suiting(TravelMode::Car, inside, also_inside);
        assert_eq!(slugs(car), ["valhalla-headway", "valhalla-master"]);

        let uncovered_transit = app_state.routers_suiting(TravelMode::Transit, inside, outside);
        assert_eq!(uncovered_transit.count(), 0);
    }
}
