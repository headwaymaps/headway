mod service;
pub use service::ElevationService;

use geo::geometry::LineString;
use geo::{Densify, Haversine};
use georaster::geotiff::{GeoTiffReader, RasterValue};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Error>;

pub trait LngLat {
    fn lng(&self) -> f64;
    fn lat(&self) -> f64;

    fn georaster_coordinate(&self) -> georaster::Coordinate {
        georaster::Coordinate::new(self.lat(), self.lng())
    }
}

impl LngLat for geo::Coord {
    fn lng(&self) -> f64 {
        self.x
    }
    fn lat(&self) -> f64 {
        self.y
    }
}

impl LngLat for geo::Point {
    fn lng(&self) -> f64 {
        self.0.x
    }
    fn lat(&self) -> f64 {
        self.0.y
    }
}

mod tiff_id {
    use super::LngLat;

    #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
    pub struct TiffId {
        lng: i16,
        lat: i8,
    }

    impl TiffId {
        pub(crate) fn for_point(lng_lat: &impl LngLat) -> Self {
            Self {
                lng: lng_lat.lng().floor() as i16,
                lat: lng_lat.lat().floor() as i8,
            }
        }

        pub(crate) fn as_string(&self) -> String {
            let lat_direction = if self.lat.is_positive() { "N" } else { "S" };
            let lng_direction = if self.lng.is_positive() { "E" } else { "W" };
            format!(
                "{lat_direction}{lat:02}{lng_direction}{lng:03}",
                lat = self.lat.abs(),
                lng = self.lng.abs()
            )
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn for_point() {
            let id = TiffId::for_point(&geo::coord!(x: 1.0, y: 2.0));
            assert_eq!(id.as_string(), "N02E001");

            let id = TiffId::for_point(&geo::coord!(x: -1.0, y: -2.0));
            assert_eq!(id.as_string(), "S02W001");

            let id = TiffId::for_point(&geo::coord!(x: 1.1, y: 2.1));
            assert_eq!(id.as_string(), "N02E001");

            let id = TiffId::for_point(&geo::coord!(x: -1.1, y: -2.1));
            assert_eq!(id.as_string(), "S03W002");
        }
    }
}

use tiff_id::TiffId;

type TiffReader = GeoTiffReader<BufReader<File>>;
pub struct Dem {
    tif_dir: PathBuf,
    geotiffs: HashMap<TiffId, TiffReader>,
}

impl Dem {
    pub fn from_dir(tif_dir: impl AsRef<Path>) -> Self {
        Self {
            tif_dir: tif_dir.as_ref().into(),
            geotiffs: HashMap::new(),
        }
    }

    pub fn geotiff(&mut self, tif_id: TiffId) -> Result<&mut TiffReader> {
        let entry = self.geotiffs.entry(tif_id);
        match entry {
            Entry::Occupied(existing) => Ok(existing.into_mut()),
            Entry::Vacant(vacant) => {
                let mut geotiff = PathBuf::from(&self.tif_dir);
                geotiff.push(format!("{}.tif", tif_id.as_string()));
                // dbg!("Opening {geotiff:?}");
                let file = BufReader::new(File::open(&geotiff)?);
                let mut geotiff = GeoTiffReader::open(file)?;
                // For multi-image tiffs, geotiff crate starts on the final one.
                // This might be a bug - I reported here: https://github.com/pka/georaster/issues/13
                geotiff.seek_to_image(0)?;
                log::debug!("Inserting {tif_id:?} {:?}", geotiff.image_info());
                Ok(vacant.insert(geotiff))
            }
        }
    }

    pub fn elevation(&mut self, point: &impl LngLat) -> Result<i16> {
        let coordinate = point.georaster_coordinate();

        let tif_id = TiffId::for_point(point);

        match self.geotiff(tif_id)?.read_pixel_at_location(coordinate) {
            RasterValue::NoData => todo!("NoData"),
            RasterValue::U8(_) => todo!("U8"),
            RasterValue::U16(_) => todo!("U16"),
            RasterValue::U32(_) => todo!("U32"),
            RasterValue::U64(_) => todo!("U64"),
            RasterValue::F32(_) => todo!("F32"),
            RasterValue::F64(_) => todo!("F64"),
            RasterValue::I8(_) => todo!("I8"),
            RasterValue::I16(v) => Ok(v),
            RasterValue::I32(_) => todo!("I32"),
            RasterValue::I64(_) => todo!("I64"),
            RasterValue::Rgb8(_, _, _) => todo!("RGB8"),
            RasterValue::Rgba8(_, _, _, _) => todo!("RGBa8"),
            RasterValue::Rgb16(_, _, _) => todo!("Rgb16"),
            RasterValue::Rgba16(_, _, _, _) => todo!("Rgba16"),
            _ => todo!("non-exhaustive"),
        }
    }

    pub fn elevations(&mut self, lng_lats: &[impl LngLat]) -> Result<Vec<i16>> {
        lng_lats
            .iter()
            .map(|lng_lat| self.elevation(lng_lat))
            .collect::<Result<Vec<_>>>()
    }

    /// Densifies `line_string` and gets elevation at every point.
    ///
    /// The output has an elevation for every coordinate in LineString
    pub fn sample_elevations(
        &mut self,
        line_string: &LineString,
        max_sample_meters: f64,
    ) -> Result<(LineString, Vec<i16>)> {
        let densified = Haversine.densify(line_string, max_sample_meters);
        let elevations = self.elevations(&densified.0)?;
        Ok((densified, elevations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dem() -> Dem {
        Dem::from_dir("tests/fixtures/low_res_elevation_tifs")
    }

    #[test]
    fn point_elevation() {
        let space_needle = geo::wkt!(POINT(-122.3493 47.6205 ));
        let queen_anne = geo::wkt!(POINT(-122.35461 47.63437));

        let mut elevation = dem();
        assert_eq!(elevation.elevation(&space_needle).unwrap(), 37);
        assert_eq!(elevation.elevation(&queen_anne).unwrap(), 129);
    }

    #[test]
    fn line_elevation() {
        let space_needle = geo::wkt!(POINT(-122.3493 47.6205 ));
        let queen_anne = geo::wkt!(POINT(-122.35461 47.63437));

        let space_needle_to_queen_anne = geo::line_string![space_needle.0, queen_anne.0];
        let mut elevation = dem();

        let expected = [
            37, 37, 42, 32, 32, 32, 46, 46, 46, 99, 99, 111, 139, 139, 139, 129, 129,
        ];
        let (_, actual) = elevation
            .sample_elevations(&space_needle_to_queen_anne, 100.0)
            .unwrap();
        assert_eq!(actual[0], 37);
        assert_eq!(*actual.last().unwrap(), 129);

        assert_eq!(actual, expected);
    }

    #[test]
    fn crossing_lng() {
        env_logger::init();

        let phöben = geo::wkt!(POINT(12.88298 52.42644));
        let berlin_center = geo::wkt!(POINT(13.405022 52.518451));

        let mut elevation = dem();
        assert_eq!(elevation.elevation(&phöben).unwrap(), 41);
        assert_eq!(elevation.elevation(&berlin_center).unwrap(), 38);

        let phöben_to_berlin_center = geo::line_string![phöben.0, berlin_center.0];

        let (_, actual) = elevation
            .sample_elevations(&phöben_to_berlin_center, 2000.0)
            .unwrap();
        assert_eq!(actual[0], 41);
        assert_eq!(*actual.last().unwrap(), 38);

        let expected = [
            41, 42, 45, 30, 28, 26, 36, 56, 46, 45, 50, 30, 49, 47, 53, 46, 40, 37, 35, 38,
        ];
        assert_eq!(actual, expected);
    }
}
