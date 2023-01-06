use transitmux::{Cluster, Error, Result};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    cluster: Cluster,
}

impl AppState {
    pub async fn add_endpoint(&mut self, endpoint: &str) -> Result<()> {
        log::info!("adding endpoint: {endpoint}");
        let url = Url::parse(endpoint).map_err(|err| {
            log::error!("error while parsing endpoint url {endpoint:?}");
            Error::server(format!("invalid endpoint url: {err}"))
        })?;

        // TODO: Separate inserting an endpoint from (periodically) fetching its routers
        self.cluster.insert_endpoint(url).await.map_err(|err| {
            log::error!("error while inserting endpoint {endpoint:?}");
            err
        })?;
        log::info!("added endpoint: {endpoint}");
        Ok(())
    }
    pub fn cluster(&self) -> &Cluster {
        &self.cluster
    }
}
