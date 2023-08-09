use gtfout::{
    geom::{Point, Rect},
    Result,
};

use std::path::PathBuf;

use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct GTFSPoint {
    shape_pt_lon: f64,
    shape_pt_lat: f64,
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1);
    let gtfs_dirs: Vec<PathBuf> = args.map(PathBuf::from).collect();
    assert!(!gtfs_dirs.is_empty(), "must specify gtfs dirs");
    let bbox = compute_bbox(gtfs_dirs)?;
    println!("{}", bbox.bbox_fmt());
    Ok(())
}

fn compute_bbox(gtfs_dirs: Vec<PathBuf>) -> Result<Rect> {
    let mut bbox: Option<Rect> = None;
    let mut first_point: Option<Point> = None;
    let mut num_points = 0;

    let mut expand_bbox = |mut reader: csv::Reader<_>| -> Result<()> {
        for result in reader.deserialize() {
            let record: GTFSPoint = result?;
            let point = Point::new(record.shape_pt_lon, record.shape_pt_lat);
            num_points += 1;

            match (&mut bbox, &mut first_point) {
                (Some(bbox), _) => bbox.expand(point),
                (None, None) => first_point = Some(point),
                (None, Some(first_point)) => bbox = Some(Rect::new(point, *first_point)),
            }
        }
        Ok(())
    };

    for mut shape_path in gtfs_dirs {
        shape_path.push("shapes.txt");
        eprintln!("parsing {shape_path:?}");
        let reader = csv::Reader::from_path(shape_path)?;
        expand_bbox(reader)?;
    }

    eprintln!("completed bbox calculation of {num_points} points");
    Ok(bbox.expect("bbox must be computed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let actual = compute_bbox(vec!["data/mock_gtfs_1".into()]).unwrap();
        let expected = Rect::new(
            Point::new(-122.366249, 47.599201),
            Point::new(-122.281769, 47.64312),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_2() {
        let actual = compute_bbox(vec!["data/mock_gtfs_2".into()]).unwrap();
        let expected = Rect::new(
            Point::new(-118.4467287702, 34.0636633497),
            Point::new(-118.4371927733, 34.0764316858),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_1_and_2() {
        let actual =
            compute_bbox(vec!["data/mock_gtfs_1".into(), "data/mock_gtfs_2".into()]).unwrap();
        let expected = Rect::new(
            Point::new(-122.366249, 34.0636633497),
            Point::new(-118.4371927733, 47.64312),
        );
        assert_eq!(actual, expected);
    }
}
