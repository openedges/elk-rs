use crate::org::eclipse::elk::core::util::IDataObject;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

impl Spacing {
    pub fn new() -> Self {
        Spacing {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
        }
    }

    pub fn with_values(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Spacing {
            top,
            bottom,
            left,
            right,
        }
    }

    pub fn set(&mut self, top: f64, right: f64, bottom: f64, left: f64) {
        self.top = top;
        self.right = right;
        self.bottom = bottom;
        self.left = left;
    }

    pub fn set_from(&mut self, other: &Spacing) {
        self.set(other.top, other.right, other.bottom, other.left);
    }

    pub fn get_top(&self) -> f64 {
        self.top
    }

    pub fn set_top(&mut self, top: f64) {
        self.top = top;
    }

    pub fn get_right(&self) -> f64 {
        self.right
    }

    pub fn set_right(&mut self, right: f64) {
        self.right = right;
    }

    pub fn get_bottom(&self) -> f64 {
        self.bottom
    }

    pub fn set_bottom(&mut self, bottom: f64) {
        self.bottom = bottom;
    }

    pub fn get_left(&self) -> f64 {
        self.left
    }

    pub fn set_left(&mut self, left: f64) {
        self.left = left;
    }

    pub fn set_left_right(&mut self, value: f64) {
        self.left = value;
        self.right = value;
    }

    pub fn set_top_bottom(&mut self, value: f64) {
        self.top = value;
        self.bottom = value;
    }

    pub fn get_horizontal(&self) -> f64 {
        self.left + self.right
    }

    pub fn get_vertical(&self) -> f64 {
        self.top + self.bottom
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self::new()
    }
}

impl IDataObject for Spacing {}

