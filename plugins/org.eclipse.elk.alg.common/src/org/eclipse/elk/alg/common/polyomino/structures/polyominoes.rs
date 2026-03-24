use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::{MapPropertyHolder, Property};

use crate::org::eclipse::elk::alg::common::polyomino::ProfileFill;

use super::{PlanarGrid, PolyominoLike};

pub struct Polyominoes<P: PolyominoLike> {
    properties: MapPropertyHolder,
    polys: Vec<P>,
    grid: PlanarGrid,
}

impl<P: PolyominoLike> Polyominoes<P> {
    pub fn new<I: IntoIterator<Item = P>>(polys: I, aspect_ratio: f64, fill: bool) -> Self {
        let mut poly_vec: Vec<P> = Vec::new();
        let mut grid_width: usize = 0;
        let mut grid_height: usize = 0;

        for mut poly in polys {
            if fill {
                ProfileFill::fill_polyomino(&mut poly);
            }
            grid_width += poly.get_width();
            grid_height += poly.get_height();
            poly_vec.push(poly);
        }

        if let Some(poly) = poly_vec.first() {
            grid_width += poly.get_width();
            grid_height += poly.get_height();
        }

        grid_width *= 2;
        grid_height *= 2;

        if aspect_ratio > 1.0 {
            grid_width = ((grid_width as f64) * aspect_ratio).ceil() as usize;
        } else if aspect_ratio > 0.0 {
            grid_height = ((grid_height as f64) / aspect_ratio).ceil() as usize;
        }

        Polyominoes {
            properties: MapPropertyHolder::new(),
            polys: poly_vec,
            grid: PlanarGrid::new(grid_width, grid_height),
        }
    }

    pub fn get_polyominoes(&self) -> &Vec<P> {
        &self.polys
    }

    pub fn get_polyominoes_mut(&mut self) -> &mut Vec<P> {
        &mut self.polys
    }

    pub fn get_grid(&self) -> &PlanarGrid {
        &self.grid
    }

    pub fn get_grid_mut(&mut self) -> &mut PlanarGrid {
        &mut self.grid
    }

    pub fn parts_mut(&mut self) -> (&mut Vec<P>, &mut PlanarGrid) {
        (&mut self.polys, &mut self.grid)
    }

    pub fn properties(&self) -> &MapPropertyHolder {
        &self.properties
    }

    pub fn properties_mut(&mut self) -> &mut MapPropertyHolder {
        &mut self.properties
    }

    pub fn get_property<T: Clone + Send + Sync + 'static>(
        &self,
        property: &Property<T>,
    ) -> Option<T> {
        self.properties.get_property(property)
    }

    pub fn set_property<T: Clone + Send + Sync + 'static>(
        &mut self,
        property: &Property<T>,
        value: Option<T>,
    ) {
        self.properties.set_property(property, value);
    }

    pub fn into_parts(self) -> (Vec<P>, PlanarGrid) {
        (self.polys, self.grid)
    }
}
