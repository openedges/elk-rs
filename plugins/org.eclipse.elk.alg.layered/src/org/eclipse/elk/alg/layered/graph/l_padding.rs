#[derive(Clone, Debug, PartialEq)]
pub struct LPadding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl LPadding {
    pub fn new() -> Self {
        LPadding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

impl Default for LPadding {
    fn default() -> Self {
        Self::new()
    }
}
