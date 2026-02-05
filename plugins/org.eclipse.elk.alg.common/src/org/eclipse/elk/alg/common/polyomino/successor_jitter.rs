use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;
use crate::org::eclipse::elk::alg::common::polyomino::successor::SuccessorFunction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub struct SuccessorJitter;

impl<P: PolyominoLike> SuccessorFunction<P> for SuccessorJitter {
    fn apply(&mut self, coords: Pair<i32, i32>, _poly: &P) -> Pair<i32, i32> {
        let x = coords.first;
        let y = coords.second;
        let cost = x.abs().max(y.abs());
        let (new_x, new_y) = if x <= 0 && x == y {
            (0, y - 1)
        } else if x == -cost && y != cost {
            let mut next_x = y;
            let next_y = x;
            if y >= 0 {
                next_x += 1;
            }
            (next_x, next_y)
        } else {
            (-y, x)
        };

        Pair::of(new_x, new_y)
    }
}
