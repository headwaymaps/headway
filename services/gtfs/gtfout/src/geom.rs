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

    pub fn bbox_fmt(&self) -> String {
        let left = self.min.x();
        let bottom = self.min.y();
        let right = self.max.x();
        let top = self.max.y();

        format!("{left} {bottom} {right} {top}")
    }
}
