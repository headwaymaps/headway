use geo::Point;
use url::Url;

use super::valhalla_api::{ModeCosting, ValhallaRouteQuery};
use crate::{DistanceUnit, Result};

#[derive(Debug, Clone)]
pub struct ValhallaRouter {
    endpoint: Url,
}

impl ValhallaRouter {
    pub fn new(endpoint: Url) -> Self {
        Self { endpoint }
    }

    pub fn plan_url(
        &self,
        source: Point,
        destination: Point,
        mode: ModeCosting,
        num_itineraries: u32,
        distance_units: DistanceUnit,
    ) -> Result<Url> {
        let mut url = self.endpoint.clone();

        let query = ValhallaRouteQuery {
            locations: vec![source.into(), destination.into()],
            costing: mode,
            alternates: num_itineraries,
            // NOTE: these units get embedded in the localized turn-by-turn direction strings
            units: distance_units,
        };

        url.set_path("/route");

        let json_string = serde_json::to_string(&query).expect("valid json representation");
        url.query_pairs_mut().append_pair("json", &json_string);

        Ok(url)
    }
}
