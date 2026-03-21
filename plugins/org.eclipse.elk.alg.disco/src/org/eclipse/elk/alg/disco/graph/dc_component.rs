use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    elk_rectangle::ElkRectangle, kvector::KVector,
};

use super::{DCElementRef, DCExtension};

pub struct DCComponent {
    offset: KVector,
    shapes: Vec<DCElementRef>,
    changed: bool,
    bounds: KVector,
    min_corner: KVector,
    id: i32,
}

impl DCComponent {
    pub fn new() -> Self {
        DCComponent {
            offset: KVector::new(),
            shapes: Vec::new(),
            changed: true,
            bounds: KVector::new(),
            min_corner: KVector::new(),
            id: -1,
        }
    }

    pub fn set_offset(&mut self, offset: KVector) {
        self.changed = true;
        self.offset = offset;
    }

    pub fn get_offset(&self) -> KVector {
        self.offset
    }

    pub fn get_elements(&self) -> &Vec<DCElementRef> {
        &self.shapes
    }

    pub fn get_dimensions_of_bounding_rectangle(&mut self) -> KVector {
        if self.changed {
            self.update();
        }
        self.bounds
    }

    pub fn get_min_corner(&mut self) -> KVector {
        if self.changed {
            self.update();
        }
        self.min_corner
    }

    pub fn intersects(&mut self, rect: &ElkRectangle) -> bool {
        for elem in &self.shapes {
            if let Some(elem_guard) = elem.lock_ok() {
                if elem_guard.intersects(rect) {
                    return true;
                }
            }
        }
        false
    }

    pub fn add_element(&mut self, element: DCElementRef) {
        self.changed = true;
        self.shapes.push(element);
    }

    pub fn add_elements(&mut self, elements: Vec<DCElementRef>) {
        for elem in elements {
            self.add_element(elem);
        }
    }

    pub fn get_id(&self) -> i32 {
        self.id
    }

    pub fn set_id(&mut self, id: i32) {
        self.id = id;
    }

    fn update(&mut self) {
        self.changed = false;
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for elem in &self.shapes {
            let elem_guard = match elem.lock_ok() {
            Some(guard) => guard,
            None => continue,
            };
            let elem_bounds = elem_guard.get_bounds();
            min_x = min_x.min(elem_bounds.x);
            max_x = max_x.max(elem_bounds.x + elem_bounds.width);
            min_y = min_y.min(elem_bounds.y);
            max_y = max_y.max(elem_bounds.y + elem_bounds.height);

            for ext in elem_guard.get_extensions() {
                update_bounds_with_extension(
                    ext,
                    &elem_bounds,
                    &mut min_x,
                    &mut max_x,
                    &mut min_y,
                    &mut max_y,
                );
            }
        }

        self.bounds = KVector::with_values(max_x - min_x, max_y - min_y);
        self.min_corner = KVector::with_values(min_x + self.offset.x, min_y + self.offset.y);
    }
}

impl Default for DCComponent {
    fn default() -> Self {
        Self::new()
    }
}

fn update_bounds_with_extension(
    ext: &DCExtension,
    elem_bounds: &ElkRectangle,
    min_x: &mut f64,
    max_x: &mut f64,
    min_y: &mut f64,
    max_y: &mut f64,
) {
    let dir = ext.get_direction();
    if dir.is_horizontal() {
        let min_pos = elem_bounds.y + ext.get_offset().y;
        let max_pos = min_pos + ext.get_width();
        *min_y = (*min_y).min(min_pos);
        *max_y = (*max_y).max(max_pos);
    } else {
        let min_pos = elem_bounds.x + ext.get_offset().x;
        let max_pos = min_pos + ext.get_width();
        *min_x = (*min_x).min(min_pos);
        *max_x = (*max_x).max(max_pos);
    }
}
