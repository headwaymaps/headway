use crate::otp_api;
use crate::Error;
use crate::Result;

use geo::geometry::Polygon;
use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub struct Router {
    endpoint: Url,
    router_id: String,
    polygon: Polygon,
}

impl Router {
    pub fn new(endpoint: Url, router_id: String, polygon: Polygon) -> Self {
        Self {
            endpoint,
            router_id,
            polygon,
        }
    }

    pub fn polygon(&self) -> &Polygon {
        &self.polygon
    }
}

#[derive(Debug)]
pub struct RouterClient {
    endpoint: Url,
    http_client: reqwest::Client,
}

impl RouterClient {
    pub fn new(endpoint: Url) -> Self {
        let http_client = reqwest::Client::new();
        Self {
            endpoint,
            http_client,
        }
    }

    pub async fn fetch_all(&self) -> Result<Vec<Router>> {
        let response = self.http_client.get(self.endpoint.clone()).send().await?;

        if !response.status().is_success() {
            return Err(Error::server(format!(
                "HTTP error when fetching routers: {}",
                response.status()
            )));
        }

        let otp_routers: otp_api::Routers = response.json().await?;
        Ok(otp_routers
            .router_info
            .into_iter()
            .map(|router| Router::new(self.endpoint.clone(), router.router_id, router.polygon))
            .collect())
    }

    pub fn router_url(router: &Router) -> Url {
        let base_path = router.endpoint.path();
        let router_path = format!("{base_path}/{}/plan", router.router_id);
        let mut router_url = router.endpoint.clone();
        router_url.set_path(&router_path);
        router_url
    }
}
