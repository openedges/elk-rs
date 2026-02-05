use std::any::Any;

use crate::org::eclipse::elk::alg::layered::graph::LNodeRef;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;

pub trait ICrossingMinimizationHeuristic: IInitializable + Send {
    fn always_improves(&self) -> bool;

    fn set_first_layer_order(&mut self, order: &mut [Vec<LNodeRef>], forward_sweep: bool) -> bool;

    fn minimize_crossings(
        &mut self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
        is_first_sweep: bool,
    ) -> bool;

    fn is_deterministic(&self) -> bool;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}
