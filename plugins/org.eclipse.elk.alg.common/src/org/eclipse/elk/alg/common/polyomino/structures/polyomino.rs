use crate::org::eclipse::elk::alg::common::utils::UniqueTriple;

use super::{Direction, PlanarGrid};

pub trait PolyominoLike {
    fn grid(&self) -> &PlanarGrid;
    fn grid_mut(&mut self) -> &mut PlanarGrid;
    fn get_polyomino_extensions(&self) -> &Vec<UniqueTriple<Direction, i32, i32>>;
    fn set_x(&mut self, value: i32);
    fn set_y(&mut self, value: i32);
    fn get_x(&self) -> i32;
    fn get_y(&self) -> i32;

    fn get_width(&self) -> usize {
        self.grid().get_width()
    }

    fn get_height(&self) -> usize {
        self.grid().get_height()
    }

    fn get_center_x(&self) -> i32 {
        self.grid().get_center_x()
    }

    fn get_center_y(&self) -> i32 {
        self.grid().get_center_y()
    }

    fn is_empty(&self, x: usize, y: usize) -> bool {
        self.grid().is_empty(x, y)
    }

    fn is_blocked(&self, x: usize, y: usize) -> bool {
        self.grid().is_blocked(x, y)
    }

    fn is_weakly_blocked(&self, x: usize, y: usize) -> bool {
        self.grid().is_weakly_blocked(x, y)
    }

    fn set_blocked(&mut self, x: usize, y: usize) {
        self.grid_mut().set_blocked(x, y)
    }

    fn set_weakly_blocked(&mut self, x: usize, y: usize) {
        self.grid_mut().set_weakly_blocked(x, y)
    }

    fn set_empty(&mut self, x: usize, y: usize) {
        self.grid_mut().set_empty(x, y)
    }

    fn add_extension(&mut self, dir: Direction, offset: i32, width: i32);
}

#[derive(Clone, Debug)]
pub struct Polyomino {
    grid: PlanarGrid,
    x: i32,
    y: i32,
    polyomino_extensions: Vec<UniqueTriple<Direction, i32, i32>>,
}

impl Polyomino {
    pub fn new(width: usize, height: usize) -> Self {
        Polyomino {
            grid: PlanarGrid::new(width, height),
            x: 0,
            y: 0,
            polyomino_extensions: Vec::new(),
        }
    }

    pub fn with_extensions(
        width: usize,
        height: usize,
        extensions: Vec<UniqueTriple<Direction, i32, i32>>,
    ) -> Self {
        Polyomino {
            grid: PlanarGrid::new(width, height),
            x: 0,
            y: 0,
            polyomino_extensions: extensions,
        }
    }

    pub fn reinitialize(&mut self, width: usize, height: usize) {
        self.grid.reinitialize(width, height);
    }
}

impl Default for Polyomino {
    fn default() -> Self {
        Polyomino::new(0, 0)
    }
}

impl PolyominoLike for Polyomino {
    fn grid(&self) -> &PlanarGrid {
        &self.grid
    }

    fn grid_mut(&mut self) -> &mut PlanarGrid {
        &mut self.grid
    }

    fn get_polyomino_extensions(&self) -> &Vec<UniqueTriple<Direction, i32, i32>> {
        &self.polyomino_extensions
    }

    fn set_x(&mut self, value: i32) {
        self.x = value;
    }

    fn set_y(&mut self, value: i32) {
        self.y = value;
    }

    fn get_x(&self) -> i32 {
        self.x
    }

    fn get_y(&self) -> i32 {
        self.y
    }

    fn add_extension(&mut self, dir: Direction, offset: i32, width: i32) {
        self.polyomino_extensions
            .push(UniqueTriple::new(dir, offset, width));
    }
}
