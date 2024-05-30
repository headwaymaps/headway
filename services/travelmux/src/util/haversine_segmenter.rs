use geo::{
    algorithm::{HaversineDistance, HaversineIntermediate},
    geometry::{Coord, LineString, Point},
};

pub struct HaversineSegmenter {
    geometry: LineString,
    next_index: usize,
}

impl HaversineSegmenter {
    pub fn new(geometry: LineString) -> Self {
        Self {
            geometry,
            next_index: 0,
        }
    }
    pub fn next_segment(&mut self, distance_meters: f64) -> Option<LineString> {
        if self.geometry.0.is_empty() {
            return None;
        }

        if self.next_index == self.geometry.0.len() - 1 {
            return None;
        }
        let mut distance_remaining = distance_meters;
        let mut start = self.geometry.0[self.next_index];
        let mut output = vec![start];
        while self.next_index < self.geometry.0.len() - 1 {
            let end = self.geometry.0[self.next_index + 1];
            let segment_length = Point::from(start).haversine_distance(&Point::from(end));
            if segment_length > distance_remaining {
                // take whatever portion of the segment we can fit
                let ratio = distance_remaining / segment_length;
                let intermediate =
                    Point::from(start).haversine_intermediate(&Point::from(end), ratio);
                output.push(Coord::from(intermediate));
                if self.geometry.0[self.next_index] == Coord::from(intermediate) {
                    debug_assert!(
                        false,
                        "intermediate point is the same as the start point - inifinite loop?"
                    );
                    // skip a point rather than risk infinite loop
                    self.next_index += 1;
                }
                // overwrite the last point with the intermediate value
                self.geometry.0[self.next_index] = Coord::from(intermediate);
                break;
            }

            output.push(end);
            distance_remaining -= segment_length;
            start = end;
            self.next_index += 1;
        }
        Some(LineString::new(output))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_relative_eq;
    use geo::{coord, HaversineLength};

    #[test]
    fn test_happy_path() {
        // paris to berlin (878km) to prague
        let paris = coord!(x: 2.3514, y: 48.8575);
        let berlin = coord!(x: 13.4050, y: 52.5200);
        let prague = coord!(x: 14.4378, y: 50.0755);

        let paris_to_berlin_distance = LineString::new(vec![paris, berlin]).haversine_length();
        assert_relative_eq!(paris_to_berlin_distance, 877461.0, epsilon = 1.0);

        let line_string = LineString::new(vec![paris, berlin, prague]);
        let total_distance = line_string.haversine_length();
        assert_relative_eq!(total_distance, 1_158_595.0, epsilon = 1.0);

        let mut segmenter = HaversineSegmenter::new(line_string);

        let east_of_paris = coord!(x: 2.467660089582291, y: 48.90485360250366);
        let segment_1 = segmenter.next_segment(10_000.0).unwrap();
        assert_relative_eq!(segment_1.haversine_length(), 10_000.0, epsilon = 1e-9);
        assert_relative_eq!(segment_1, LineString::new(vec![paris, east_of_paris]));

        // next one should pick up where the last one left off
        let segment_2 = segmenter.next_segment(10_000.0).unwrap();
        assert_eq!(segment_1.0.last().unwrap(), segment_2.0.first().unwrap());

        let east_of_berlin = coord!(x: 13.482210264987538, y: 52.34640526357316);
        let segment_3 = segmenter.next_segment(paris_to_berlin_distance).unwrap();
        let expected = LineString::new(vec![*segment_2.0.last().unwrap(), berlin, east_of_berlin]);
        assert_relative_eq!(segment_3, expected);

        // overshoot it
        let next = segmenter.next_segment(total_distance).unwrap();
        assert_relative_eq!(next, LineString::new(vec![east_of_berlin, prague]));

        let next = segmenter.next_segment(4.0);
        assert!(next.is_none());
    }

    #[test]
    fn degenerate_empty_line_string() {
        let empty = LineString::new(vec![]);
        assert_eq!(0.0, empty.haversine_length());

        let mut segmenter = HaversineSegmenter::new(empty);
        let first = segmenter.next_segment(10.0);
        assert!(first.is_none());
    }

    #[test]
    fn degenerate_line_string_with_1_pt() {
        let empty = LineString::new(vec![coord!(x: 1.0, y: 2.0)]);
        assert_eq!(0.0, empty.haversine_length());

        let mut segmenter = HaversineSegmenter::new(empty);
        let first = segmenter.next_segment(10.0);
        assert!(first.is_none());
    }
}
