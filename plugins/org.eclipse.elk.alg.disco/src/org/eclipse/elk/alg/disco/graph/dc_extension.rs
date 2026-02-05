use org_eclipse_elk_core::org::eclipse::elk::core::math::{elk_rectangle::ElkRectangle, kvector::KVector};

use super::DCDirection;

#[derive(Clone, Debug)]
pub struct DCExtension {
    direction: DCDirection,
    offset: KVector,
    width: f64,
}

impl DCExtension {
    pub fn new(parent_bounds: &ElkRectangle, direction: DCDirection, middle_pos: &KVector, width: f64) -> Self {
        let mut offset = KVector::with_values(-parent_bounds.x, -parent_bounds.y);
        offset.add(middle_pos);
        let half_width = width / 2.0;
        if direction.is_horizontal() {
            offset.sub_values(0.0, half_width);
        } else {
            offset.sub_values(half_width, 0.0);
        }
        DCExtension {
            direction,
            offset,
            width,
        }
    }

    pub fn get_direction(&self) -> DCDirection {
        self.direction
    }

    pub fn get_offset(&self) -> KVector {
        self.offset
    }

    pub fn get_width(&self) -> f64 {
        self.width
    }
}
