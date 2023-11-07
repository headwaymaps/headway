mod chained_reader;
mod chained_zip_reader;

use chained_reader::ChainedReader;

use flatgeobuf::{FgbWriter, GeometryType};
use geozero::geojson::GeoJsonLineReader;
use geozero::GeozeroDatasource;

use std::fs::File;
use std::io::{BufWriter, Read};
use std::path::PathBuf;

use crate::chained_zip_reader::ChainedZipReader;
use openaddresses::Result;

#[macro_use]
extern crate log;

const USAGE: &str = "required arguments: <input glob> <output path>";

fn main() -> Result<()> {
    pretty_env_logger::init_timed();
    let args: Vec<_> = std::env::args().collect();
    let input_glob = args.get(1).expect(USAGE);
    let output_path = args.get(2).expect(USAGE);

    let mut input: Box<dyn Read> = if input_glob.ends_with(".zip") {
        debug!("opening reader for zip file: {input_glob}");
        Box::new(ChainedZipReader::new(&PathBuf::from(input_glob))?)
    } else {
        debug!("opening chained reader for file glob: {input_glob}");
        let mut input_paths: Vec<_> = glob::glob(input_glob)
            .into_iter()
            .flatten()
            .flat_map(|entry| {
                let entry = entry.expect("valid directory entry");
                if entry.metadata().expect("valid metadata").is_dir() {
                    return None;
                }
                Some(entry)
            })
            .collect();
        input_paths.sort();
        input_paths.reverse();
        Box::new(ChainedReader::new(input_paths))
    };

    let output_file = File::create(output_path)?;
    let mut output = BufWriter::new(output_file);

    let mut reader = GeoJsonLineReader::new(&mut input);
    let mut writer = FgbWriter::create("fgb", GeometryType::Point)?;

    info!("start processing");
    reader.process(&mut writer)?;
    info!("done processing");
    info!("start writing");
    writer.write(&mut output)?;
    info!("done writing");
    Ok(())
}
