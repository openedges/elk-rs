use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::p2processingorder::{MaxSTPhase, MinSTPhase};
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TreeConstructionStrategy {
    MinimumSpanningTree,
    MaximumSpanningTree,
}

impl TreeConstructionStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            TreeConstructionStrategy::MinimumSpanningTree => 0,
            TreeConstructionStrategy::MaximumSpanningTree => 1,
        }
    }
}

impl Default for TreeConstructionStrategy {
    fn default() -> Self {
        TreeConstructionStrategy::MinimumSpanningTree
    }
}

impl ILayoutPhaseFactory<SPOrEPhases, Graph> for TreeConstructionStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<SPOrEPhases, Graph>> {
        match self {
            TreeConstructionStrategy::MinimumSpanningTree => Box::new(MinSTPhase::new()),
            TreeConstructionStrategy::MaximumSpanningTree => Box::new(MaxSTPhase::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
