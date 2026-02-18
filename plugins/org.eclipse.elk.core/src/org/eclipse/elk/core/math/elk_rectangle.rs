use crate::org::eclipse::elk::core::math::kvector::KVector;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElkRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ElkRectangle {
    pub fn new() -> Self {
        ElkRectangle {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn with_values(x: f64, y: f64, width: f64, height: f64) -> Self {
        ElkRectangle {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_other(rect: &ElkRectangle) -> Self {
        *rect
    }

    pub fn set_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }

    pub fn get_position(&self) -> KVector {
        KVector::with_values(self.x, self.y)
    }

    pub fn get_top_left(&self) -> KVector {
        self.get_position()
    }

    pub fn get_top_right(&self) -> KVector {
        KVector::with_values(self.x + self.width, self.y)
    }

    pub fn get_bottom_left(&self) -> KVector {
        KVector::with_values(self.x, self.y + self.height)
    }

    pub fn get_bottom_right(&self) -> KVector {
        KVector::with_values(self.x + self.width, self.y + self.height)
    }

    pub fn get_center(&self) -> KVector {
        KVector::with_values(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn union(&mut self, other: &ElkRectangle) {
        let mut x1 = self.x.min(other.x);
        let mut y1 = self.y.min(other.y);
        let mut x2 = (self.x + self.width).max(other.x + other.width);
        let mut y2 = (self.y + self.height).max(other.y + other.height);
        if x2 < x1 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y2 < y1 {
            std::mem::swap(&mut y1, &mut y2);
        }
        self.set_rect(x1, y1, x2 - x1, y2 - y1);
    }

    pub fn move_by(&mut self, offset: &KVector) {
        self.x += offset.x;
        self.y += offset.y;
    }

    pub fn get_max_x(&self) -> f64 {
        self.x + self.width
    }

    pub fn get_max_y(&self) -> f64 {
        self.y + self.height
    }

    pub fn intersects(&self, rect: &ElkRectangle) -> bool {
        let r1x1 = self.x;
        let r1y1 = self.y;
        let r1x2 = self.x + self.width;
        let r1y2 = self.y + self.height;
        let r2x1 = rect.x;
        let r2y1 = rect.y;
        let r2x2 = rect.x + rect.width;
        let r2y2 = rect.y + rect.height;

        r1x1 < r2x2 && r1x2 > r2x1 && r1y2 > r2y1 && r1y1 < r2y2
    }
}

impl Default for ElkRectangle {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ElkRectangle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rect[x={},y={},w={},h={}]",
            self.x, self.y, self.width, self.height
        )
    }
}
