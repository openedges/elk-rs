use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;
use crate::org::eclipse::elk::alg::common::polyomino::successor::SuccessorFunction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub struct SuccessorMaxNormWindingInMathPosSense;

impl<P: PolyominoLike> SuccessorFunction<P> for SuccessorMaxNormWindingInMathPosSense {
    fn apply(&mut self, coords: Pair<i32, i32>, _poly: &P) -> Pair<i32, i32> {
        let x = coords.first;
        let y = coords.second;
        let cost = x.abs().max(y.abs());
        if x < cost && y == -cost {
            return Pair::of(x + 1, y);
        }
        if x == cost && y < cost {
            return Pair::of(x, y + 1);
        }
        if x >= -cost && y == cost {
            return Pair::of(x - 1, y);
        }
        Pair::of(x, y - 1)
    }
}
