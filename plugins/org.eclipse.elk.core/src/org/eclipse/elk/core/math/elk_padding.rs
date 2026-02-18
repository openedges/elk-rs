use crate::org::eclipse::elk::core::math::spacing::Spacing;
use crate::org::eclipse::elk::core::util::IDataObject;

#[derive(Clone, Debug, PartialEq)]
pub struct ElkPadding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl ElkPadding {
    pub fn new() -> Self {
        ElkPadding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub fn with_any(any: f64) -> Self {
        ElkPadding {
            top: any,
            right: any,
            bottom: any,
            left: any,
        }
    }

    pub fn with_sides(left_right: f64, top_bottom: f64) -> Self {
        ElkPadding {
            top: top_bottom,
            right: left_right,
            bottom: top_bottom,
            left: left_right,
        }
    }

    pub fn with_values(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        ElkPadding {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn from_other(other: &ElkPadding) -> Self {
        ElkPadding {
            top: other.top,
            right: other.right,
            bottom: other.bottom,
            left: other.left,
        }
    }

    pub fn spacing(&self) -> Spacing {
        Spacing::with_values(self.top, self.right, self.bottom, self.left)
    }
}

impl Default for ElkPadding {
    fn default() -> Self {
        Self::new()
    }
}

impl IDataObject for ElkPadding {}
