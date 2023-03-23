use geom::{Rect, Point};

use std::path::PathBuf;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct GTFSPoint {
    shape_pt_lon: f64,
    shape_pt_lat: f64
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1);
    let gtfs_dirs: Vec<PathBuf> = args.map(PathBuf::from).collect();
    assert!(gtfs_dirs.len() > 0, "must specify gtfs dirs");
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
                (None, Some(first_point)) => bbox = Some(Rect::new(point, *first_point))
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


mod geom {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Point {
        x: f64, y: f64
    }

    impl Point {
        pub fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }

        pub fn x(&self) -> f64 {
            self.x
        }

        pub fn y(&self) -> f64 {
            self.y
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Rect { min: Point, max: Point }

    impl Rect {
        pub fn new(a: Point, b: Point) -> Self {
            let min_x = a.x().min(b.x());
            let max_x = a.x().max(b.x());
            let min_y = a.y().min(b.y());
            let max_y = a.y().max(b.y());

            let min = Point::new(min_x, min_y);
            let max = Point::new(max_x, max_y);

            Self { min, max }
        }

        pub fn expand(&mut self, point: Point) {
            if point.x() > self.max.x() {
                self.max.x = point.x();
            } else if point.x() < self.min.x() {
                self.min.x = point.x();
            }

            if point.y() > self.max.y() {
                self.max.y = point.y();
            } else if point.y() < self.min.y() {
                self.min.y = point.y();
            }
        }

        pub fn bbox_fmt(&self) -> String {
            let left = self.min.x();
            let bottom = self.min.y();
            let right = self.max.x();
            let top = self.max.y();

            format!("{left} {bottom} {right} {top}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let actual = compute_bbox(vec!["data/mock_gtfs_1".into()]).unwrap();
        let expected = Rect::new(Point::new(-122.366249, 47.599201), Point::new(-122.281769, 47.64312));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_2() {
        let actual = compute_bbox(vec!["data/mock_gtfs_2".into()]).unwrap();
        let expected = Rect::new(Point::new(-118.4467287702, 34.0636633497), Point::new(-118.4371927733, 34.0764316858));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_1_and_2() {
        let actual = compute_bbox(vec!["data/mock_gtfs_1".into(), "data/mock_gtfs_2".into()]).unwrap();
        let expected = Rect::new(Point::new(-122.366249, 34.0636633497), Point::new(-118.4371927733, 47.64312));
        assert_eq!(actual, expected);
    }
}

