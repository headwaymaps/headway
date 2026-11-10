use crate::otp::PreparedOTPRouter;
use crate::valhalla::ValhallaRouter;
use crate::{Error, Result, TravelMode};
use geo::Point;

/// Note this is *not* `Send`: the OTP coverage areas it holds are `Rc`-backed, so each server
/// worker builds its own from a shared [`crate::otp::OTPRouter`].
#[derive(Debug, Clone)]
pub struct AppState {
    /// By slug, in the order to try them for a trip that names none. An OTP slug can name a router
    /// per zone.
    routers: Vec<(String, Router)>,
}

#[derive(Debug, Clone)]
pub enum Router {
    Otp(Box<PreparedOTPRouter>),
    Valhalla(ValhallaRouter),
}

impl Router {
    pub fn covers(&self, from: Point, to: Point) -> bool {
        match self {
            Router::Otp(otp) => otp.contains(&from) && otp.contains(&to),
            Router::Valhalla(_) => true,
        }
    }

    /// Whether to offer this router a trip that doesn't name one.
    fn suits(&self, mode: TravelMode, from: Point, to: Point) -> bool {
        let plans_mode = match self {
            Router::Otp(_) => mode != TravelMode::Car,
            Router::Valhalla(_) => mode != TravelMode::Transit,
        };
        plans_mode && self.covers(from, to)
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

    pub fn routers_named(&self, slug: &str) -> Result<Vec<&Router>> {
        let routers: Vec<&Router> = self
            .routers
            .iter()
            .filter(|(s, _)| s == slug)
            .map(|(_, router)| router)
            .collect();
        if routers.is_empty() {
            return Err(Error::user(format!("no router is called {slug}")));
        }
        Ok(routers)
    }

    pub fn otp_routers(&self) -> impl Iterator<Item = &PreparedOTPRouter> {
        self.routers.iter().filter_map(|(_, router)| match router {
            Router::Otp(otp) => Some(otp.as_ref()),
            Router::Valhalla(_) => None,
        })
    }

    /// The first OTP instance covering both ends of the trip.
    pub fn otp_router(&self, from: Point, to: Point) -> Option<&PreparedOTPRouter> {
        self.otp_routers()
            .find(|otp| otp.contains(&from) && otp.contains(&to))
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
    use crate::otp::OTPRouter;
    use geo::{wkt, Polygon};

    fn valhalla(slug: &str) -> (String, Router) {
        let url = format!("http://{slug}:8002").parse().unwrap();
        (slug.to_string(), Router::Valhalla(ValhallaRouter::new(url)))
    }

    fn otp(slug: &str, zone: Polygon) -> (String, Router) {
        let url = format!("http://{slug}:8080").parse().unwrap();
        let otp = OTPRouter::new("Zone".to_string(), url, zone, None);
        (slug.to_string(), Router::Otp(Box::new(otp.prepare())))
    }

    fn app_state() -> AppState {
        AppState::new(vec![
            otp(
                "otp-headway",
                wkt! { POLYGON((0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0, 0.0 0.0)) },
            ),
            otp(
                "otp-headway",
                wkt! { POLYGON((20.0 20.0, 30.0 20.0, 30.0 30.0, 20.0 30.0, 20.0 20.0)) },
            ),
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
        let otp_zones = app_state.routers_named("otp-headway").unwrap();
        assert_eq!(otp_zones.len(), 2);
        let Ok([Router::Valhalla(router)]) = app_state.routers_named("valhalla-master").as_deref()
        else {
            panic!("expected valhalla-master");
        };
        assert_eq!(router.endpoint().host_str(), Some("valhalla-master"));
        assert!(app_state.routers_named("osrm").is_err());
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

        let other_zone = Point::new(21.0, 21.0);
        let across_zones = app_state.routers_suiting(TravelMode::Transit, inside, other_zone);
        assert_eq!(across_zones.count(), 0);

        let uncovered_transit = app_state.routers_suiting(TravelMode::Transit, inside, outside);
        assert_eq!(uncovered_transit.count(), 0);
    }
}
