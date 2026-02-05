use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub trait SuccessorFunction<P: PolyominoLike> {
    fn apply(&mut self, coords: Pair<i32, i32>, poly: &P) -> Pair<i32, i32>;
}
