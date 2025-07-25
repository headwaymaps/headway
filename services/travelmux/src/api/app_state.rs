use crate::elevation::ElevationService;
use crate::{otp::OtpCluster, valhalla::ValhallaRouter, Error, Result};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone)]
pub struct AppState {
    otp_cluster: OtpCluster,
    valhalla_router: ValhallaRouter,
    elevation: ElevationService,
}

impl AppState {
    pub fn new(valhalla_endpoint: Url, tif_dir: PathBuf) -> Self {
        log::info!("new AppState with valhalla_endpoint: {valhalla_endpoint:?}");
        let valhalla_router = ValhallaRouter::new(valhalla_endpoint);
        debug_assert!(std::fs::exists(&tif_dir).unwrap());
        Self {
            valhalla_router,
            otp_cluster: OtpCluster::default(),
            elevation: ElevationService::new(tif_dir),
        }
    }

    pub async fn add_otp_endpoint(&mut self, endpoint: &str) -> Result<()> {
        log::info!("adding endpoint: {endpoint}");
        let url = Url::parse(endpoint).map_err(|err| {
            log::error!("error while parsing endpoint url {endpoint:?}");
            Error::server(format!("invalid endpoint url: {err}"))
        })?;

        // TODO: Separate inserting an endpoint from (periodically) fetching its routers
        self.otp_cluster
            .insert_endpoint(url)
            .await
            .inspect_err(|err| {
                log::error!("error while inserting endpoint {endpoint:?}, {err}");
            })?;
        log::info!("added endpoint: {endpoint}");
        Ok(())
    }

    pub fn otp_cluster(&self) -> &OtpCluster {
        &self.otp_cluster
    }

    pub fn valhalla_router(&self) -> &ValhallaRouter {
        &self.valhalla_router
    }

    pub fn elevation(&self) -> &ElevationService {
        &self.elevation
    }
}
