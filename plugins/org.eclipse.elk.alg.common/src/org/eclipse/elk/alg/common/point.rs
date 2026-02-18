use std::hash::{Hash, Hasher};

use org_eclipse_elk_core::org::eclipse::elk::core::math::KVector;

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub quadrant: Quadrant,
    pub convex: bool,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point {
            x,
            y,
            quadrant: Quadrant::Q1,
            convex: true,
        }
    }

    pub fn with_quadrant(x: f64, y: f64, quadrant: Quadrant) -> Self {
        Point {
            x,
            y,
            quadrant,
            convex: true,
        }
    }

    pub fn from(v: &KVector) -> Self {
        Point::new(v.x, v.y)
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for Point {}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Quadrant {
    Q1,
    Q4,
    Q2,
    Q3,
}

impl Quadrant {
    pub fn is_upper(self) -> bool {
        matches!(self, Quadrant::Q1 | Quadrant::Q2)
    }

    pub fn is_left(self) -> bool {
        matches!(self, Quadrant::Q1 | Quadrant::Q4)
    }

    pub fn is_both_left_or_both_right(q1: Quadrant, q2: Quadrant) -> bool {
        matches!(
            (q1, q2),
            (Quadrant::Q1, Quadrant::Q4)
                | (Quadrant::Q4, Quadrant::Q1)
                | (Quadrant::Q3, Quadrant::Q2)
                | (Quadrant::Q2, Quadrant::Q3)
        )
    }

    pub fn is_one_left_one_right(q1: Quadrant, q2: Quadrant) -> bool {
        matches!(
            (q1, q2),
            (Quadrant::Q1, Quadrant::Q2)
                | (Quadrant::Q1, Quadrant::Q3)
                | (Quadrant::Q4, Quadrant::Q3)
                | (Quadrant::Q4, Quadrant::Q2)
        )
    }
}
