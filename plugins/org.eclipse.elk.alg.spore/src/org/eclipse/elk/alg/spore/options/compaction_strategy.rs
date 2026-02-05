use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::p3execution::ShrinkTreeCompactionPhase;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CompactionStrategy {
    DepthFirst,
}

impl CompactionStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            CompactionStrategy::DepthFirst => 0,
        }
    }
}

impl Default for CompactionStrategy {
    fn default() -> Self {
        CompactionStrategy::DepthFirst
    }
}

impl ILayoutPhaseFactory<SPOrEPhases, Graph> for CompactionStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<SPOrEPhases, Graph>> {
        match self {
            CompactionStrategy::DepthFirst => Box::new(ShrinkTreeCompactionPhase::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
