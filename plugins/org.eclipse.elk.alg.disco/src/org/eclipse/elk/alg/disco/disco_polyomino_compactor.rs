use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::compaction::options::PolyominoOptions;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::polyomino::PolyominoCompactor;
use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::polyomino::structures::{
    PolyominoLike, Polyominoes,
};
use org_eclipse_elk_core::org::eclipse::elk::core::math::{elk_padding::ElkPadding, kvector::KVector};
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use crate::org::eclipse::elk::alg::disco::graph::DCGraph;
use crate::org::eclipse::elk::alg::disco::i_compactor::ICompactor;
use crate::org::eclipse::elk::alg::disco::options::DisCoOptions;
use crate::org::eclipse::elk::alg::disco::structures::DCPolyomino;

pub struct DisCoPolyominoCompactor {
    upper_bound: f64,
}

impl DisCoPolyominoCompactor {
    pub fn new() -> Self {
        DisCoPolyominoCompactor { upper_bound: 100.0 }
    }

    fn compute_cell_size(&self, graph: &mut DCGraph) -> f64 {
        let mut sum_term = 0.0;
        let mut prod_term = 0.0;
        let comps = graph.components().clone();
        let num_of_comps = comps.len() as f64;

        for comp in comps {
            let mut comp_guard = comp.lock().expect("component lock");
            let bounds = comp_guard.get_dimensions_of_bounding_rectangle();
            let width = bounds.x;
            let height = bounds.y;
            sum_term += width + height;
            prod_term += width * height;
        }

        let four = 4.0;
        let numerator =
            (four * self.upper_bound * num_of_comps * prod_term - four * prod_term + sum_term * sum_term)
                .sqrt()
                + sum_term;
        let denominator = 2.0 * (self.upper_bound * num_of_comps - 1.0);
        if denominator == 0.0 {
            return numerator;
        }
        numerator / denominator
    }

    fn create_polyominoes(
        &self,
        graph: &mut DCGraph,
        polys: &mut Vec<DCPolyomino>,
        cell_size_x: f64,
        cell_size_y: f64,
    ) {
        for comp in graph.components() {
            polys.push(DCPolyomino::new(comp.clone(), cell_size_x, cell_size_y));
        }
    }

    fn pack_polyominoes(
        &self,
        mut polys: Vec<DCPolyomino>,
        aspect_ratio: f64,
        fill: bool,
    ) -> (Vec<DCPolyomino>, org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::polyomino::structures::PlanarGrid) {
        for (id, poly) in polys.iter_mut().enumerate() {
            poly.set_id(id as i32);
        }

        let mut poly_holder = Polyominoes::new(polys, aspect_ratio, fill);
        let compactor = PolyominoCompactor::new();
        compactor.pack_polyominoes(&mut poly_holder);
        poly_holder.into_parts()
    }

    fn apply_to_dc_graph(
        &self,
        graph: &mut DCGraph,
        polys: &Vec<DCPolyomino>,
        grid: &org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::polyomino::structures::PlanarGrid,
        grid_cell_size_x: f64,
        grid_cell_size_y: f64,
    ) {
        let grid_crop = grid.get_filled_bounds();
        let padding = graph
            .properties_mut()
            .get_property(DisCoOptions::PADDING)
            .unwrap_or_else(ElkPadding::new);
        let padding_hori = padding.left + padding.right;
        let padding_vert = padding.top + padding.bottom;
        let parent_width = (*grid_crop.third() as f64) * grid_cell_size_x + padding_hori;
        let parent_height = (*grid_crop.fourth() as f64) * grid_cell_size_y + padding_vert;
        graph.set_dimensions(KVector::with_values(parent_width, parent_height));

        for poly in polys {
            let absolute_int_x = poly.get_x() - *grid_crop.first();
            let absolute_int_y = poly.get_y() - *grid_crop.second();

            let mut absolute_position = KVector::with_values(absolute_int_x as f64, absolute_int_y as f64);
            absolute_position.scale_values(poly.get_cell_size_x(), poly.get_cell_size_y());
            absolute_position.add(&poly.get_offset());

            let mut comp_guard = poly.get_representee().lock().expect("component lock");
            let original_coordinates = comp_guard.get_min_corner();
            let mut offset = KVector::from_vector(&absolute_position);
            offset.sub(&original_coordinates);
            comp_guard.set_offset(offset);
        }
    }
}

impl Default for DisCoPolyominoCompactor {
    fn default() -> Self {
        Self::new()
    }
}

impl ICompactor for DisCoPolyominoCompactor {
    fn compact(&mut self, graph: &mut DCGraph) {
        let mut polys: Vec<DCPolyomino> = Vec::new();

        let mut grid_cell_size_x = self.compute_cell_size(graph);
        let mut grid_cell_size_y = grid_cell_size_x;

        let fill = graph
            .properties_mut()
            .get_property(PolyominoOptions::POLYOMINO_FILL)
            .unwrap_or(false);
        let mut aspect_ratio = graph
            .properties_mut()
            .get_property(CoreOptions::ASPECT_RATIO)
            .unwrap_or(1.0);
        if aspect_ratio == 0.0 {
            aspect_ratio = 1.0;
        }

        if aspect_ratio > 1.0 {
            grid_cell_size_x *= aspect_ratio;
        } else {
            grid_cell_size_y /= aspect_ratio;
        }

        self.create_polyominoes(graph, &mut polys, grid_cell_size_x, grid_cell_size_y);
        let (polys, grid) = self.pack_polyominoes(polys, aspect_ratio, fill);
        self.apply_to_dc_graph(graph, &polys, &grid, grid_cell_size_x, grid_cell_size_y);

        graph
            .properties_mut()
            .set_property(DisCoOptions::DEBUG_DISCO_POLYS, Some(polys));
    }
}
