use geo::geometry::Polygon;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Router {
    #[serde(deserialize_with = "geojson::de::deserialize_geometry")]
    pub polygon: Polygon,
    pub router_id: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Routers {
    pub router_info: Vec<Router>,
}
