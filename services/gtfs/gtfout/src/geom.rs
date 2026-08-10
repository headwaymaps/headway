#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
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
pub struct Rect {
    min: Point,
    max: Point,
}

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

    pub fn min(&self) -> Point {
        self.min
    }

    pub fn max(&self) -> Point {
        self.max
    }

    /// Whether the two rectangles share any area. Touching edges count.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.min.x() <= other.max.x()
            && self.max.x() >= other.min.x()
            && self.min.y() <= other.max.y()
            && self.max.y() >= other.min.y()
    }

    pub fn bbox_fmt(&self) -> String {
        let left = self.min.x();
        let bottom = self.min.y();
        let right = self.max.x();
        let top = self.max.y();

        format!("{left} {bottom} {right} {top}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rect {
        Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
    }

    #[test]
    fn overlapping_rects_intersect() {
        assert!(rect(0.0, 0.0, 2.0, 2.0).intersects(&rect(1.0, 1.0, 3.0, 3.0)));
    }

    #[test]
    fn contained_rect_intersects() {
        assert!(rect(0.0, 0.0, 10.0, 10.0).intersects(&rect(4.0, 4.0, 5.0, 5.0)));
        assert!(rect(4.0, 4.0, 5.0, 5.0).intersects(&rect(0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn touching_edges_intersect() {
        assert!(rect(0.0, 0.0, 1.0, 1.0).intersects(&rect(1.0, 0.0, 2.0, 1.0)));
    }

    #[test]
    fn disjoint_rects_do_not_intersect() {
        // Apart in x only, y only, and both.
        assert!(!rect(0.0, 0.0, 1.0, 1.0).intersects(&rect(2.0, 0.0, 3.0, 1.0)));
        assert!(!rect(0.0, 0.0, 1.0, 1.0).intersects(&rect(0.0, 2.0, 1.0, 3.0)));
        assert!(!rect(0.0, 0.0, 1.0, 1.0).intersects(&rect(5.0, 5.0, 6.0, 6.0)));
    }
}
