use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;
use crate::org::eclipse::elk::alg::common::polyomino::successor::SuccessorFunction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub struct SuccessorManhattan;

impl<P: PolyominoLike> SuccessorFunction<P> for SuccessorManhattan {
    fn apply(&mut self, coords: Pair<i32, i32>, _poly: &P) -> Pair<i32, i32> {
        let x = coords.first;
        let y = coords.second;
        let (mut new_x, mut new_y) = (x, y);

        if x == 0 && y == 0 {
            new_y -= 1;
        } else if x == -1 && y <= 0 {
            new_x = 0;
            new_y -= 2;
        } else if x <= 0 && y > 0 {
            new_x -= 1;
            new_y -= 1;
        } else if x >= 0 && y < 0 {
            new_x += 1;
            new_y += 1;
        } else if x > 0 && y >= 0 {
            new_x -= 1;
            new_y += 1;
        } else {
            new_x += 1;
            new_y -= 1;
        }

        Pair::of(new_x, new_y)
    }
}
