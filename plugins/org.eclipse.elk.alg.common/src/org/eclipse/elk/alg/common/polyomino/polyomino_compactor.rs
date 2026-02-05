use std::cmp::Ordering;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

use crate::org::eclipse::elk::alg::common::compaction::options::{
    HighLevelSortingCriterion, LowLevelSortingCriterion, PolyominoOptions, TraversalStrategy,
};
use crate::org::eclipse::elk::alg::common::polyomino::successor::{
    SuccessorFunction,
};
use crate::org::eclipse::elk::alg::common::polyomino::structures::{
    Direction, PolyominoLike, Polyominoes,
};
use crate::org::eclipse::elk::alg::common::polyomino::{
    SuccessorCombination, SuccessorJitter, SuccessorLineByLine, SuccessorManhattan,
    SuccessorMaxNormWindingInMathPosSense, SuccessorQuadrantsGeneric,
};

pub struct PolyominoCompactor;

impl PolyominoCompactor {
    pub fn new() -> Self {
        PolyominoCompactor
    }

    pub fn pack_polyominoes<P: PolyominoLike + 'static>(&self, poly_holder: &mut Polyominoes<P>) {
        let low_level = poly_holder
            .get_property(PolyominoOptions::POLYOMINO_LOW_LEVEL_SORT)
            .unwrap_or_default();
        let high_level = poly_holder
            .get_property(PolyominoOptions::POLYOMINO_HIGH_LEVEL_SORT)
            .unwrap_or_default();
        let traversal = poly_holder
            .get_property(PolyominoOptions::POLYOMINO_TRAVERSAL_STRATEGY)
            .unwrap_or_default();

        {
            let polys = poly_holder.get_polyominoes_mut();
            match low_level {
                LowLevelSortingCriterion::BySize => polys.sort_by(min_perimeter_cmp),
                LowLevelSortingCriterion::BySizeAndShape => polys.sort_by(min_perimeter_shape_cmp),
            }

            match high_level {
                HighLevelSortingCriterion::CornerCasesThanSingleSideLast => {
                    polys.sort_by(min_num_extensions_cmp);
                    polys.sort_by(single_extension_side_cmp);
                    polys.sort_by(corner_cases_cmp);
                }
                HighLevelSortingCriterion::NumOfExternalSidesThanNumOfExtensionsLast => {
                    polys.sort_by(min_num_extensions_cmp);
                    polys.sort_by(min_num_extension_directions_cmp);
                }
            }
        }

        let mut successor: Box<dyn SuccessorFunction<P>> = match traversal {
            TraversalStrategy::Spiral => Box::new(SuccessorMaxNormWindingInMathPosSense),
            TraversalStrategy::LineByLine => Box::new(SuccessorLineByLine),
            TraversalStrategy::Manhattan => Box::new(SuccessorManhattan),
            TraversalStrategy::Jitter => Box::new(SuccessorJitter),
            TraversalStrategy::QuadrantsManhattan => {
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorManhattan)))
            }
            TraversalStrategy::QuadrantsLineByLine => {
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorLineByLine)))
            }
            TraversalStrategy::QuadrantsJitter => {
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorJitter)))
            }
            TraversalStrategy::CombineLineByLineManhattan => Box::new(SuccessorCombination::new(
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorLineByLine))),
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorManhattan))),
            )),
            TraversalStrategy::CombineJitterManhattan => Box::new(SuccessorCombination::new(
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorJitter))),
                Box::new(SuccessorQuadrantsGeneric::new(Box::new(SuccessorManhattan))),
            )),
        };

        let (polys, grid) = poly_holder.parts_mut();
        for poly in polys.iter_mut() {
            let mut off_x = 0;
            let mut off_y = 0;
            let mut next = Pair::of(off_x, off_y);
            while grid.intersects_with_center_based_polyomino(poly, off_x, off_y) {
                next = successor.apply(next, poly);
                off_x = next.first;
                off_y = next.second;
            }
            grid.add_filled_cells_from_polyomino(poly, off_x, off_y);
        }
    }
}

fn min_perimeter_cmp<P: PolyominoLike>(a: &P, b: &P) -> Ordering {
    let half_peri_a = a.get_width() + a.get_height();
    let half_peri_b = b.get_width() + b.get_height();
    half_peri_a.cmp(&half_peri_b)
}

fn min_perimeter_shape_cmp<P: PolyominoLike>(a: &P, b: &P) -> Ordering {
    fn value<P: PolyominoLike>(poly: &P) -> usize {
        let mut width = poly.get_width();
        let mut height = poly.get_height();
        if width < height {
            width *= width;
        } else {
            height *= height;
        }
        width + height
    }

    value(a).cmp(&value(b))
}

fn min_num_extensions_cmp<P: PolyominoLike>(a: &P, b: &P) -> Ordering {
    a.get_polyomino_extensions()
        .len()
        .cmp(&b.get_polyomino_extensions().len())
}

fn min_num_extension_directions_cmp<P: PolyominoLike>(a: &P, b: &P) -> Ordering {
    num_extension_dirs(a).cmp(&num_extension_dirs(b))
}

fn single_extension_side_cmp<P: PolyominoLike>(a: &P, b: &P) -> Ordering {
    let a_val = if num_extension_dirs(a) == 1 { 1 } else { 0 };
    let b_val = if num_extension_dirs(b) == 1 { 1 } else { 0 };
    a_val.cmp(&b_val)
}

fn corner_cases_cmp<P: PolyominoLike>(a: &P, b: &P) -> Ordering {
    let a_val = if is_corner_case(a) { 1 } else { 0 };
    let b_val = if is_corner_case(b) { 1 } else { 0 };
    a_val.cmp(&b_val)
}

fn num_extension_dirs<P: PolyominoLike>(poly: &P) -> usize {
    let mut dirs: Vec<Direction> = Vec::new();
    for ext in poly.get_polyomino_extensions() {
        let dir = *ext.first();
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs.len()
}

fn is_corner_case<P: PolyominoLike>(poly: &P) -> bool {
    let mut dirs: Vec<Direction> = Vec::new();
    for ext in poly.get_polyomino_extensions() {
        let dir = *ext.first();
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    if dirs.len() != 2 {
        return false;
    }
    let horizontal_count = dirs.iter().filter(|dir| dir.is_horizontal()).count();
    horizontal_count == 1
}
