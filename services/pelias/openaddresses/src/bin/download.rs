use openaddresses::{Error, Result};
use std::fs::File;
use std::io::BufWriter;

use flatgeobuf::HttpFgbReader;

use clap::Parser;

#[derive(Parser)]
struct Args {
    /// Path to HTTP hosted indexed FGB file
    input_url: String,

    /// Where to write the resultant CSV file
    output_path: String,

    /// Fetch a subset of the FGB file based on a bounding box. If not specified, fetches all the
    /// data.
    ///
    /// format: <left bottom right top>
    ///
    ///   e.g.: -122.462 47.394 -122.005 47.831
    #[clap(long)]
    bbox: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init_timed();
    let args = Args::parse();
    let input_url = args.input_url;
    let bbox = args.bbox.map(|s| BBox::from_str(&s).unwrap());
    let output_path = args.output_path;

    download(&input_url, bbox, &output_path).await
}

async fn download(input_url: &str, bbox: Option<BBox>, output_path: &str) -> Result<()> {
    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);
    let mut csv_writer = geozero::csv::CsvWriter::new(&mut writer);

    if input_url.ends_with(".fgb") {
        let reader = HttpFgbReader::open(input_url).await?;
        let mut reader = if let Some(bbox) = bbox {
            reader
                .select_bbox(bbox.left, bbox.bottom, bbox.right, bbox.top)
                .await?
        } else {
            reader.select_all().await?
        };
        reader.process_features(&mut csv_writer).await?;
    } else if input_url.ends_with(".geomedea") {
        let mut reader = geomedea::HttpReader::open(input_url).await?;
        let reader = if let Some(bbox) = bbox {
            let bounds = geomedea::Bounds::from(bbox);
            reader.select_bbox(&bounds).await?
        } else {
            reader.select_all().await?
        };
        todo!("reader.process_features(&mut csv_writer).await?");
    } else {
        panic!("unsupported input format: {input_url}");
    };

    Ok(())
}

//   -122.462 47.394 -122.005 47.831
// left,bottom,right,top
#[derive(Debug, Clone)]
struct BBox {
    left: f64,
    bottom: f64,
    right: f64,
    top: f64,
}

impl From<BBox> for geomedea::Bounds {
    fn from(value: BBox) -> Self {
        let a = geomedea::LngLat::degrees(value.left, value.bottom);
        let b = geomedea::LngLat::degrees(value.right, value.top);
        Self::from_corners(&a, &b)
    }
}

use std::str::FromStr;

impl FromStr for BBox {
    type Err = crate::Error;
    fn from_str(val: &str) -> Result<Self> {
        let mut floats = val
            .split(' ')
            .map(|word| f64::from_str(word.trim()).map_err(|e| format!("{e}: {word}")));
        let left = floats.next().unwrap_or_else(|| Err("bbox missing left")?)?;
        let bottom = floats
            .next()
            .unwrap_or_else(|| Err("bbox missing bottom")?)?;
        let right = floats
            .next()
            .unwrap_or_else(|| Err("bbox missing right")?)?;
        let top = floats.next().unwrap_or_else(|| Err("bbox missing top")?)?;
        if floats.next().is_some() {
            Err("Too many numbers in BBox")?;
        }
        Ok(Self {
            left,
            bottom,
            right,
            top,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bbox() {
        let bbox = BBox::from_str("-122.462 47.394 -122.005 47.831").expect("valid bbox");
        assert_eq!(bbox.left, -122.462);
        assert_eq!(bbox.bottom, 47.394);
        assert_eq!(bbox.right, -122.005);
        assert_eq!(bbox.top, 47.831);
    }
}
