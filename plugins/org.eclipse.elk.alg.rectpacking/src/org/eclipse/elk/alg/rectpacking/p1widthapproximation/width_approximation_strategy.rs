use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;

use super::{GreedyWidthApproximator, TargetWidthWidthApproximator};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum WidthApproximationStrategy {
    #[default]
    Greedy,
    TargetWidth,
}

impl WidthApproximationStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            WidthApproximationStrategy::Greedy => 0,
            WidthApproximationStrategy::TargetWidth => 1,
        }
    }
}

impl ILayoutPhaseFactory<RectPackingLayoutPhases, ElkNodeRef> for WidthApproximationStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef>> {
        match self {
            WidthApproximationStrategy::Greedy => Box::new(GreedyWidthApproximator::new()),
            WidthApproximationStrategy::TargetWidth => Box::new(TargetWidthWidthApproximator::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
