use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::polyomino::structures::{
    Direction, PlanarGrid, Polyomino, PolyominoLike,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::{
    elk_rectangle::ElkRectangle, kvector::KVector,
};

use crate::org::eclipse::elk::alg::disco::graph::{DCComponentRef, DCDirection, DCElement};

#[derive(Clone)]
pub struct DCPolyomino {
    poly: Polyomino,
    representee: DCComponentRef,
    p_width: usize,
    p_height: usize,
    cell_size_x: f64,
    cell_size_y: f64,
}

impl DCPolyomino {
    pub fn new(comp: DCComponentRef, cell_size_x: f64, cell_size_y: f64) -> Self {
        let mut comp_guard = comp.lock().expect("component lock");
        let comp_dims = comp_guard.get_dimensions_of_bounding_rectangle();
        let p_width = compute_low_res_dimension(comp_dims.x, cell_size_x);
        let p_height = compute_low_res_dimension(comp_dims.y, cell_size_y);
        drop(comp_guard);

        let mut poly = Polyomino::new(p_width, p_height);
        poly.reinitialize(p_width, p_height);

        let mut result = DCPolyomino {
            poly,
            representee: comp,
            p_width,
            p_height,
            cell_size_x,
            cell_size_y,
        };
        result.fill_cells();
        result.add_extensions();
        result
    }

    pub fn get_offset(&self) -> KVector {
        let mut comp_guard = self.representee.lock().expect("component lock");
        let mut offset = comp_guard.get_dimensions_of_bounding_rectangle();
        offset.sub_values(
            self.p_width as f64 * self.cell_size_x,
            self.p_height as f64 * self.cell_size_y,
        );
        offset.scale(-0.5);
        offset
    }

    pub fn get_min_corner_on_canvas(&self) -> KVector {
        let mut comp_guard = self.representee.lock().expect("component lock");
        let mut corner = comp_guard.get_min_corner();
        corner.sub(&self.get_offset());
        corner
    }

    pub fn get_cell_size_x(&self) -> f64 {
        self.cell_size_x
    }

    pub fn get_cell_size_y(&self) -> f64 {
        self.cell_size_y
    }

    pub fn get_id(&self) -> i32 {
        let comp_guard = self.representee.lock().expect("component lock");
        comp_guard.get_id()
    }

    pub fn set_id(&mut self, id: i32) {
        let mut comp_guard = self.representee.lock().expect("component lock");
        comp_guard.set_id(id);
    }

    pub fn get_representee(&self) -> &DCComponentRef {
        &self.representee
    }

    fn fill_cells(&mut self) {
        let mut blocked_cells: Vec<(usize, usize)> = Vec::new();
        {
            let mut comp_guard = self.representee.lock().expect("component lock");
            let comp_corner = comp_guard.get_min_corner();
            let mut polyo_offset = comp_guard.get_dimensions_of_bounding_rectangle();
            polyo_offset.sub_values(
                self.p_width as f64 * self.cell_size_x,
                self.p_height as f64 * self.cell_size_y,
            );
            polyo_offset.scale(-0.5);

            let base_x = comp_corner.x - polyo_offset.x;
            let mut cur_y = comp_corner.y - polyo_offset.y;

            for y in 0..self.p_height {
                let mut cur_x = base_x;
                for x in 0..self.p_width {
                    let rect =
                        ElkRectangle::with_values(cur_x, cur_y, self.cell_size_x, self.cell_size_y);
                    if comp_guard.intersects(&rect) {
                        blocked_cells.push((x, y));
                    }
                    cur_x += self.cell_size_x;
                }
                cur_y += self.cell_size_y;
            }
        }

        for (x, y) in blocked_cells {
            self.set_blocked(x, y);
        }
    }

    fn add_extensions(&mut self) {
        let comp_guard = self.representee.lock().expect("component lock");
        let elements = comp_guard.get_elements().clone();
        drop(comp_guard);

        for elem_ref in elements {
            let elem_guard = elem_ref.lock().expect("element lock");
            if !elem_guard.get_extensions().is_empty() {
                self.add_extensions_to_poly(&elem_guard);
            }
        }
    }

    fn add_extensions_to_poly(&mut self, elem: &DCElement) {
        let mut comp_guard = self.representee.lock().expect("component lock");
        let comp_corner = comp_guard.get_min_corner();
        drop(comp_guard);

        let polyo_offset = self.get_offset();
        let base_x = comp_corner.x - polyo_offset.x;
        let base_y = comp_corner.y - polyo_offset.y;

        let elem_pos = elem.get_bounds();
        let base_x = elem_pos.x - base_x;
        let base_y = elem_pos.y - base_y;

        for extension in elem.get_extensions() {
            let pos = extension.get_offset();
            let xe = base_x + pos.x;
            let ye = base_y + pos.y;

            let xp = (xe / self.cell_size_x) as i32;
            let yp = (ye / self.cell_size_y) as i32;

            let dir = extension.get_direction();
            let poly_dir = match dir {
                DCDirection::North => Direction::North,
                DCDirection::East => Direction::East,
                DCDirection::South => Direction::South,
                DCDirection::West => Direction::West,
            };

            if dir.is_horizontal() {
                let yp_plus_width = ((ye + extension.get_width()) / self.cell_size_y) as i32;
                self.add_extension(poly_dir, yp, yp_plus_width);
                if dir == DCDirection::West {
                    self.weakly_block_area(0, yp, xp, yp_plus_width);
                } else {
                    self.weakly_block_area(xp, yp, (self.p_width as i32) - 1, yp_plus_width);
                }
            } else {
                let xp_plus_width = ((xe + extension.get_width()) / self.cell_size_x) as i32;
                self.add_extension(poly_dir, xp, xp_plus_width);
                if dir == DCDirection::North {
                    self.weakly_block_area(xp, 0, xp_plus_width, yp);
                } else {
                    self.weakly_block_area(xp, yp, xp_plus_width, (self.p_height as i32) - 1);
                }
            }
        }
    }

    fn weakly_block_area(
        &mut self,
        x_upper_left: i32,
        y_upper_left: i32,
        x_bottom_right: i32,
        y_bottom_right: i32,
    ) {
        self.poly.grid_mut().weakly_block_area(
            x_upper_left,
            y_upper_left,
            x_bottom_right,
            y_bottom_right,
        );
    }
}

impl PolyominoLike for DCPolyomino {
    fn grid(&self) -> &PlanarGrid {
        self.poly.grid()
    }

    fn grid_mut(&mut self) -> &mut PlanarGrid {
        self.poly.grid_mut()
    }

    fn get_polyomino_extensions(
        &self,
    ) -> &Vec<
        org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::utils::UniqueTriple<
            Direction,
            i32,
            i32,
        >,
    > {
        self.poly.get_polyomino_extensions()
    }

    fn set_x(&mut self, value: i32) {
        self.poly.set_x(value);
    }

    fn set_y(&mut self, value: i32) {
        self.poly.set_y(value);
    }

    fn get_x(&self) -> i32 {
        self.poly.get_x()
    }

    fn get_y(&self) -> i32 {
        self.poly.get_y()
    }

    fn add_extension(&mut self, dir: Direction, offset: i32, width: i32) {
        self.poly.add_extension(dir, offset, width);
    }
}

fn compute_low_res_dimension(dim: f64, cell_size: f64) -> usize {
    let cell_fit = dim / cell_size;
    let fit_truncated = cell_fit.trunc() as usize;
    if cell_fit > fit_truncated as f64 {
        fit_truncated + 1
    } else {
        fit_truncated
    }
}
