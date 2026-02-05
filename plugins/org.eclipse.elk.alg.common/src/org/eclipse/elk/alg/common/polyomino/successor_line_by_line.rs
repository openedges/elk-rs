use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;
use crate::org::eclipse::elk::alg::common::polyomino::successor::SuccessorFunction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub struct SuccessorLineByLine;

impl<P: PolyominoLike> SuccessorFunction<P> for SuccessorLineByLine {
    fn apply(&mut self, coords: Pair<i32, i32>, _poly: &P) -> Pair<i32, i32> {
        let x = coords.first;
        let y = coords.second;
        if x >= 0 {
            if x == y {
                return Pair::of(-x - 1, -x - 1);
            }
            if x == -y {
                return Pair::of(-x, y + 1);
            }
        }
        if x.abs() > y.abs() {
            if x < 0 {
                return Pair::of(-x, y);
            }
            return Pair::of(-x, y + 1);
        }
        Pair::of(x + 1, y)
    }
}
