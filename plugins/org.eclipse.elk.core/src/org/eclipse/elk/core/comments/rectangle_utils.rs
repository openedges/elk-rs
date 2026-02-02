use crate::org::eclipse::elk::core::math::ElkRectangle;

pub const OUT_LEFT: i32 = 1;
pub const OUT_TOP: i32 = 2;
pub const OUT_RIGHT: i32 = 4;
pub const OUT_BOTTOM: i32 = 8;

pub const LEFT: i32 = OUT_LEFT;
pub const TOP: i32 = OUT_TOP;
pub const RIGHT: i32 = OUT_RIGHT;
pub const BOTTOM: i32 = OUT_BOTTOM;

pub const TOP_LEFT: i32 = OUT_TOP | OUT_LEFT;
pub const BOTTOM_LEFT: i32 = OUT_BOTTOM | OUT_LEFT;
pub const TOP_RIGHT: i32 = OUT_TOP | OUT_RIGHT;
pub const BOTTOM_RIGHT: i32 = OUT_BOTTOM | OUT_RIGHT;

pub fn outcode(rect: &ElkRectangle, x: f64, y: f64) -> i32 {
    let mut code = 0;
    if rect.width <= 0.0 {
        code |= OUT_LEFT | OUT_RIGHT;
    } else if x < rect.x {
        code |= OUT_LEFT;
    } else if x > rect.x + rect.width {
        code |= OUT_RIGHT;
    }

    if rect.height <= 0.0 {
        code |= OUT_TOP | OUT_BOTTOM;
    } else if y < rect.y {
        code |= OUT_TOP;
    } else if y > rect.y + rect.height {
        code |= OUT_BOTTOM;
    }
    code
}
