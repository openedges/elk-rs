use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::p3execution::GrowTreePhase;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum OverlapRemovalStrategy {
    #[default]
    GrowTree,
}

impl OverlapRemovalStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            OverlapRemovalStrategy::GrowTree => 0,
        }
    }
}

impl ILayoutPhaseFactory<SPOrEPhases, Graph> for OverlapRemovalStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<SPOrEPhases, Graph>> {
        match self {
            OverlapRemovalStrategy::GrowTree => Box::new(GrowTreePhase::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
