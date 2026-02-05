use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;
use crate::org::eclipse::elk::alg::common::polyomino::successor::SuccessorFunction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Pair;

pub struct SuccessorCombination<P: PolyominoLike> {
    normal_fun: Box<dyn SuccessorFunction<P>>,
    external_fun: Box<dyn SuccessorFunction<P>>,
}

impl<P: PolyominoLike> SuccessorCombination<P> {
    pub fn new(
        normal_fun: Box<dyn SuccessorFunction<P>>,
        external_fun: Box<dyn SuccessorFunction<P>>,
    ) -> Self {
        SuccessorCombination {
            normal_fun,
            external_fun,
        }
    }
}

impl<P: PolyominoLike> SuccessorFunction<P> for SuccessorCombination<P> {
    fn apply(&mut self, coords: Pair<i32, i32>, poly: &P) -> Pair<i32, i32> {
        if poly.get_polyomino_extensions().is_empty() {
            self.normal_fun.apply(coords, poly)
        } else {
            self.external_fun.apply(coords, poly)
        }
    }
}
