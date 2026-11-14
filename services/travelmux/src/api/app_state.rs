use crate::otp::OtpRouter;
use crate::valhalla::ValhallaRouter;
use crate::{Error, Result, TravelMode};
use geo::Point;

#[derive(Debug, Clone)]
pub struct AppState {
    /// In the order to try them for a trip that names none.
    routers: Vec<Router>,
}

#[derive(Debug, Clone)]
pub enum Router {
    Otp(OtpRouter),
    Valhalla(ValhallaRouter),
}

impl Router {
    pub fn id(&self) -> &str {
        match self {
            Router::Otp(otp) => otp.id(),
            Router::Valhalla(valhalla) => valhalla.id(),
        }
    }

    pub fn covers(&self, from: Point, to: Point) -> bool {
        match self {
            Router::Otp(otp) => otp.contains(&from) && otp.contains(&to),
            Router::Valhalla(valhalla) => valhalla.contains(&from) && valhalla.contains(&to),
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
    pub fn new(routers: Vec<Router>) -> Self {
        for (i, router) in routers.iter().enumerate() {
            assert!(
                routers[..i].iter().all(|other| other.id() != router.id()),
                "two routers are called {}",
                router.id()
            );
        }
        Self { routers }
    }

    /// The routers to try, in order, for a trip that doesn't name one.
    pub fn routers_suiting(
        &self,
        mode: TravelMode,
        from: Point,
        to: Point,
    ) -> impl Iterator<Item = &Router> {
        self.routers
            .iter()
            .filter(move |router| router.suits(mode, from, to))
    }

    pub fn router(&self, router_id: &str) -> Result<&Router> {
        self.routers
            .iter()
            .find(|router| router.id() == router_id)
            .ok_or_else(|| Error::user(format!("no router is called {router_id}")))
    }

    pub fn otp_routers(&self) -> impl Iterator<Item = &OtpRouter> {
        self.routers.iter().filter_map(|router| match router {
            Router::Otp(otp) => Some(otp),
            Router::Valhalla(_) => None,
        })
    }

    /// The first OTP instance covering both ends of the trip.
    pub fn otp_router(&self, from: Point, to: Point) -> Option<&OtpRouter> {
        self.otp_routers()
            .find(|otp| otp.contains(&from) && otp.contains(&to))
    }

    /// The first Valhalla instance covering both ends of the trip.
    pub fn valhalla_router(&self, from: Point, to: Point) -> Result<&ValhallaRouter> {
        self.routers
            .iter()
            .find_map(|router| match router {
                Router::Valhalla(valhalla) if router.covers(from, to) => Some(valhalla),
                _ => None,
            })
            .ok_or_else(|| Error::user("no valhalla covers this trip"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::OtpRouter;
    use crate::valhalla::ValhallaRouter;
    use geo::{coord, MultiPolygon, Polygon, Rect};

    fn square(min: f64, max: f64) -> Polygon {
        Rect::new(coord! { x: min, y: min }, coord! { x: max, y: max }).to_polygon()
    }

    fn valhalla(id: &str, coverage: Polygon) -> Router {
        let url = format!("http://{id}:8002").parse().unwrap();
        let valhalla = ValhallaRouter::new(id.to_string(), url, MultiPolygon::new(vec![coverage]));
        Router::Valhalla(valhalla)
    }

    fn otp(id: &str, coverage: Polygon) -> Router {
        let url = format!("http://{id}:8080").parse().unwrap();
        let otp = OtpRouter::new(id.to_string(), url, coverage, None);
        Router::Otp(otp)
    }

    fn app_state() -> AppState {
        AppState::new(vec![
            otp("Seattle", square(0.0, 10.0)),
            otp("Portland", square(20.0, 30.0)),
            valhalla("valhalla-headway", square(0.0, 30.0)),
            valhalla("valhalla-master", square(0.0, 40.0)),
        ])
    }

    fn ids<'a>(routers: impl Iterator<Item = &'a Router>) -> Vec<&'a str> {
        routers.map(Router::id).collect()
    }

    #[test]
    fn requests_name_any_router_by_id() {
        let app_state = app_state();
        assert!(matches!(app_state.router("Portland"), Ok(Router::Otp(_))));
        let Ok(Router::Valhalla(router)) = app_state.router("valhalla-master") else {
            panic!("expected valhalla-master");
        };
        assert_eq!(router.endpoint().host_str(), Some("valhalla-master"));
        assert!(app_state.router("osrm").is_err());
    }

    #[test]
    #[should_panic(expected = "two routers are called Seattle")]
    fn router_ids_are_unique() {
        AppState::new(vec![
            otp("Seattle", square(0.0, 10.0)),
            valhalla("Seattle", square(0.0, 10.0)),
        ]);
    }

    #[test]
    fn unnamed_trips_try_the_routers_that_suit_them_in_order() {
        let app_state = app_state();
        let point = |xy| Point::new(xy, xy);

        let bike = app_state.routers_suiting(TravelMode::Bicycle, point(1.0), point(2.0));
        assert_eq!(
            ids(bike),
            ["Seattle", "valhalla-headway", "valhalla-master"]
        );

        let car = app_state.routers_suiting(TravelMode::Car, point(1.0), point(2.0));
        assert_eq!(ids(car), ["valhalla-headway", "valhalla-master"]);

        let beyond_one_valhalla =
            app_state.routers_suiting(TravelMode::Car, point(1.0), point(35.0));
        assert_eq!(ids(beyond_one_valhalla), ["valhalla-master"]);

        let across_zones = app_state.routers_suiting(TravelMode::Transit, point(1.0), point(21.0));
        assert_eq!(across_zones.count(), 0);
    }
}
