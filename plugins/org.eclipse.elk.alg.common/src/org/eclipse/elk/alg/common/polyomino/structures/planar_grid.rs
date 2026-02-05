use org_eclipse_elk_core::org::eclipse::elk::core::util::Quadruple;

use super::{Direction, IThreeValueGrid, PolyominoLike, TwoBitGrid};

#[derive(Clone, Debug)]
pub struct PlanarGrid {
    grid: TwoBitGrid,
    x_center: i32,
    y_center: i32,
}

impl PlanarGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let grid = TwoBitGrid::new(width, height);
        let x_center = ((width as i32) - 1) >> 1;
        let y_center = ((height as i32) - 1) >> 1;
        PlanarGrid { grid, x_center, y_center }
    }

    pub fn get_width(&self) -> usize {
        self.grid.get_width()
    }

    pub fn get_height(&self) -> usize {
        self.grid.get_height()
    }

    pub fn is_empty(&self, x: usize, y: usize) -> bool {
        self.grid.is_empty(x, y)
    }

    pub fn is_blocked(&self, x: usize, y: usize) -> bool {
        self.grid.is_blocked(x, y)
    }

    pub fn is_weakly_blocked(&self, x: usize, y: usize) -> bool {
        self.grid.is_weakly_blocked(x, y)
    }

    pub fn in_bounds(&self, x: usize, y: usize) -> bool {
        self.grid.in_bounds(x, y)
    }

    pub fn reinitialize(&mut self, width: usize, height: usize) {
        self.grid.reinitialize(width, height);
        self.x_center = ((width as i32) - 1) >> 1;
        self.y_center = ((height as i32) - 1) >> 1;
    }

    pub fn set_empty(&mut self, x: usize, y: usize) {
        self.grid.set_empty(x, y);
    }

    pub fn set_blocked(&mut self, x: usize, y: usize) {
        self.grid.set_blocked(x, y);
    }

    pub fn set_weakly_blocked(&mut self, x: usize, y: usize) {
        self.grid.set_weakly_blocked(x, y);
    }

    fn to_index(&self, x: i32, y: i32) -> (usize, usize) {
        let xi = x + self.x_center;
        let yi = y + self.y_center;
        if xi < 0 || yi < 0 {
            panic!(
                "Grid is only of size {}*{}. Requested point ({}, {}) is out of bounds.",
                self.get_width(),
                self.get_height(),
                x,
                y
            );
        }
        let xu = xi as usize;
        let yu = yi as usize;
        if !self.in_bounds(xu, yu) {
            panic!(
                "Grid is only of size {}*{}. Requested point ({}, {}) is out of bounds.",
                self.get_width(),
                self.get_height(),
                x,
                y
            );
        }
        (xu, yu)
    }

    pub fn is_empty_center_based(&self, x: i32, y: i32) -> bool {
        let (xu, yu) = self.to_index(x, y);
        self.is_empty(xu, yu)
    }

    pub fn is_blocked_center_based(&self, x: i32, y: i32) -> bool {
        let (xu, yu) = self.to_index(x, y);
        self.is_blocked(xu, yu)
    }

    pub fn is_weakly_blocked_center_based(&self, x: i32, y: i32) -> bool {
        let (xu, yu) = self.to_index(x, y);
        self.is_weakly_blocked(xu, yu)
    }

    pub fn in_bounds_center_based(&self, x: i32, y: i32) -> bool {
        let xi = x + self.x_center;
        let yi = y + self.y_center;
        if xi < 0 || yi < 0 {
            return false;
        }
        self.in_bounds(xi as usize, yi as usize)
    }

    pub fn intersects_with_center_based_grid(&self, other: &PlanarGrid, x_offset: i32, y_offset: i32) -> bool {
        for x in 0..other.get_width() {
            let x_translated = x as i32 - other.get_center_x() + x_offset;
            for y in 0..other.get_height() {
                let y_translated = y as i32 - other.get_center_y() + y_offset;
                if self.in_bounds_center_based(x_translated, y_translated)
                    && ((!other.is_empty(x, y) && self.is_blocked_center_based(x_translated, y_translated))
                        || (other.is_blocked(x, y) && !self.is_empty_center_based(x_translated, y_translated)))
                {
                    return true;
                }
            }
        }
        false
    }

    pub fn intersects_with_center_based_polyomino<P: PolyominoLike>(
        &self,
        other: &P,
        x_offset: i32,
        y_offset: i32,
    ) -> bool {
        if self.intersects_with_center_based_grid(other.grid(), x_offset, y_offset) {
            return true;
        }

        for ext in other.get_polyomino_extensions() {
            // transformations for center based coordinates
            let left_x = self.get_center_x() - other.get_center_x() + x_offset;
            let right_x = left_x + other.get_width() as i32;
            let top_y = self.get_center_y() - other.get_center_y() + y_offset;
            let bottom_y = top_y + other.get_height() as i32;
            let intersects = match *ext.first() {
                Direction::North => {
                    self.weakly_intersects_area(
                        left_x + *ext.second(),
                        0,
                        left_x + *ext.third(),
                        top_y - 1,
                    )
                }
                Direction::East => {
                    self.weakly_intersects_area(
                        right_x,
                        top_y + *ext.second(),
                        self.get_width() as i32 - 1,
                        top_y + *ext.third(),
                    )
                }
                Direction::South => {
                    self.weakly_intersects_area(
                        left_x + *ext.second(),
                        bottom_y,
                        left_x + *ext.third(),
                        self.get_height() as i32 - 1,
                    )
                }
                Direction::West => {
                    self.weakly_intersects_area(
                        0,
                        top_y + *ext.second(),
                        left_x - 1,
                        top_y + *ext.third(),
                    )
                }
            };
            if intersects {
                return true;
            }
        }

        false
    }

    pub fn add_filled_cells_from_grid(&mut self, other: &PlanarGrid, x_offset: i32, y_offset: i32) {
        for x in 0..other.get_width() {
            let x_translated = x as i32 - other.get_center_x() + x_offset;
            for y in 0..other.get_height() {
                let y_translated = y as i32 - other.get_center_y() + y_offset;
                if other.is_blocked(x, y) {
                    if !self.is_weakly_blocked_center_based(x_translated, y_translated) {
                        self.set_blocked_center_based(x_translated, y_translated);
                    }
                } else if other.is_weakly_blocked(x, y)
                    && !self.is_blocked_center_based(x_translated, y_translated)
                {
                    self.set_weakly_blocked_center_based(x_translated, y_translated);
                }
            }
        }
    }

    pub fn add_filled_cells_from_polyomino<P: PolyominoLike>(
        &mut self,
        other: &mut P,
        x_offset: i32,
        y_offset: i32,
    ) {
        self.add_filled_cells_from_grid(other.grid(), x_offset, y_offset);
        other.set_x(self.x_center - other.get_center_x() + x_offset);
        other.set_y(self.y_center - other.get_center_y() + y_offset);

        for ext in other.get_polyomino_extensions() {
            match *ext.first() {
                Direction::North => {
                    self.weakly_block_area(
                        other.get_x() + *ext.second(),
                        0,
                        other.get_x() + *ext.third(),
                        other.get_y() - 1,
                    );
                }
                Direction::East => {
                    self.weakly_block_area(
                        other.get_x() + other.get_width() as i32,
                        other.get_y() + *ext.second(),
                        self.get_width() as i32 - 1,
                        other.get_y() + *ext.third(),
                    );
                }
                Direction::South => {
                    self.weakly_block_area(
                        other.get_x() + *ext.second(),
                        other.get_y() + other.get_height() as i32,
                        other.get_x() + *ext.third(),
                        self.get_height() as i32 - 1,
                    );
                }
                Direction::West => {
                    self.weakly_block_area(
                        0,
                        other.get_y() + *ext.second(),
                        other.get_x() - 1,
                        other.get_y() + *ext.third(),
                    );
                }
            }
        }
    }

    pub fn get_filled_bounds(&self) -> Quadruple<i32, i32, i32, i32> {
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        for xi in 0..self.get_width() {
            for yi in 0..self.get_height() {
                if self.is_blocked(xi, yi) {
                    let xi = xi as i32;
                    let yi = yi as i32;
                    min_x = min_x.min(xi);
                    max_x = max_x.max(xi);
                    min_y = min_y.min(yi);
                    max_y = max_y.max(yi);
                }
            }
        }
        if min_x == i32::MAX {
            return Quadruple::new(0, 0, 0, 0);
        }
        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        Quadruple::new(min_x, min_y, width, height)
    }

    pub fn weakly_block_area(
        &mut self,
        x_upper_left: i32,
        y_upper_left: i32,
        x_bottom_right: i32,
        y_bottom_right: i32,
    ) {
        for yi in y_upper_left..=y_bottom_right {
            for xi in x_upper_left..=x_bottom_right {
                let (xu, yu) = self.to_index_non_center(xi, yi);
                if !self.is_blocked(xu, yu) {
                    self.set_weakly_blocked(xu, yu);
                }
            }
        }
    }

    pub fn weakly_intersects_area(
        &self,
        x_upper_left: i32,
        y_upper_left: i32,
        x_bottom_right: i32,
        y_bottom_right: i32,
    ) -> bool {
        for yi in y_upper_left..=y_bottom_right {
            for xi in x_upper_left..=x_bottom_right {
                let (xu, yu) = self.to_index_non_center(xi, yi);
                if self.is_blocked(xu, yu) {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_center_x(&self) -> i32 {
        self.x_center
    }

    pub fn get_center_y(&self) -> i32 {
        self.y_center
    }

    pub fn set_blocked_center_based(&mut self, x: i32, y: i32) {
        let (xu, yu) = self.to_index(x, y);
        self.set_blocked(xu, yu);
    }

    pub fn set_empty_center_based(&mut self, x: i32, y: i32) {
        let (xu, yu) = self.to_index(x, y);
        self.set_empty(xu, yu);
    }

    pub fn set_weakly_blocked_center_based(&mut self, x: i32, y: i32) {
        let (xu, yu) = self.to_index(x, y);
        self.set_weakly_blocked(xu, yu);
    }

    fn to_index_non_center(&self, x: i32, y: i32) -> (usize, usize) {
        if x < 0 || y < 0 {
            panic!(
                "Grid is only of size {}*{}. Requested point ({}, {}) is out of bounds.",
                self.get_width(),
                self.get_height(),
                x,
                y
            );
        }
        let xu = x as usize;
        let yu = y as usize;
        if !self.in_bounds(xu, yu) {
            panic!(
                "Grid is only of size {}*{}. Requested point ({}, {}) is out of bounds.",
                self.get_width(),
                self.get_height(),
                x,
                y
            );
        }
        (xu, yu)
    }
}

impl Default for PlanarGrid {
    fn default() -> Self {
        PlanarGrid::new(0, 0)
    }
}
