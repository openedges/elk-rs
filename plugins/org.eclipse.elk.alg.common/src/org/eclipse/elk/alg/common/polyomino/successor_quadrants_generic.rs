use std::collections::HashSet;

use crate::org::eclipse::elk::alg::common::polyomino::structures::{Direction, PolyominoLike};
use crate::org::eclipse::elk::alg::common::polyomino::successor::SuccessorFunction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub struct SuccessorQuadrantsGeneric<P: PolyominoLike> {
    cost_fun: Box<dyn SuccessorFunction<P>>,
    last_poly_key: Option<usize>,
    pos_x: bool,
    pos_y: bool,
    neg_x: bool,
    neg_y: bool,
}

impl<P: PolyominoLike> SuccessorQuadrantsGeneric<P> {
    pub fn new(cost_fun: Box<dyn SuccessorFunction<P>>) -> Self {
        SuccessorQuadrantsGeneric {
            cost_fun,
            last_poly_key: None,
            pos_x: true,
            pos_y: true,
            neg_x: true,
            neg_y: true,
        }
    }

    fn update_quadrants(&mut self, poly: &P) {
        let mut dir_set: HashSet<Direction> = HashSet::new();
        for ext in poly.get_polyomino_extensions() {
            dir_set.insert(*ext.first());
        }

        self.pos_x = true;
        self.pos_y = true;
        self.neg_x = true;
        self.neg_y = true;

        let contains_pos = dir_set.contains(&Direction::North);
        let contains_neg = dir_set.contains(&Direction::South);
        if contains_pos && !contains_neg {
            self.pos_y = false;
        }
        if !contains_pos && contains_neg {
            self.neg_y = false;
        }

        let contains_pos = dir_set.contains(&Direction::East);
        let contains_neg = dir_set.contains(&Direction::West);
        if contains_pos && !contains_neg {
            self.neg_x = false;
        }
        if !contains_pos && contains_neg {
            self.pos_x = false;
        }
    }
}

impl<P: PolyominoLike> SuccessorFunction<P> for SuccessorQuadrantsGeneric<P> {
    fn apply(&mut self, coords: Pair<i32, i32>, poly: &P) -> Pair<i32, i32> {
        let key = poly as *const P as usize;
        if self.last_poly_key != Some(key) {
            self.last_poly_key = Some(key);
            self.update_quadrants(poly);
        }

        let mut next_coords = self.cost_fun.apply(coords, poly);
        loop {
            let new_x = next_coords.first;
            let new_y = next_coords.second;

            let mut invalid = false;
            if new_x < 0 {
                if !self.neg_x {
                    invalid = true;
                }
            } else if !self.pos_x {
                invalid = true;
            }

            if new_y < 0 {
                if !self.neg_y {
                    invalid = true;
                }
            } else if !self.pos_y {
                invalid = true;
            }

            if !invalid {
                return next_coords;
            }
            next_coords = self.cost_fun.apply(next_coords, poly);
        }
    }
}
