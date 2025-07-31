use super::{Dem, Result};
use geo::LineString;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ElevationService {
    tif_dir: PathBuf,
}

impl ElevationService {
    pub fn new(tif_dir: PathBuf) -> Self {
        Self { tif_dir }
    }

    pub fn sample_elevations(
        &self,
        line_string: &LineString,
        max_sample_meters: f64,
    ) -> Result<(LineString, Vec<i16>)> {
        self.elevation()
            .sample_elevations(line_string, max_sample_meters)
    }

    fn elevation(&self) -> Dem {
        // TODO: pool
        Dem::from_dir(self.tif_dir.clone())
    }
}
