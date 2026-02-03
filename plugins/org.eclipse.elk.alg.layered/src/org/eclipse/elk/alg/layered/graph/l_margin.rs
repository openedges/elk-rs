#[derive(Clone, Debug, PartialEq)]
pub struct LMargin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl LMargin {
    pub fn new() -> Self {
        LMargin {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl Default for LMargin {
    fn default() -> Self {
        Self::new()
    }
}
