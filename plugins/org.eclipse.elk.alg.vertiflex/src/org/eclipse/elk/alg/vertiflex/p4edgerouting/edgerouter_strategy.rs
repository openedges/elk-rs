use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::p4edgerouting::{BendEdgeRouter, StraightEdgeRouter};
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum EdgerouterStrategy {
    #[default]
    DirectRouting,
    BendRouting,
}

impl EdgerouterStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            EdgerouterStrategy::DirectRouting => 0,
            EdgerouterStrategy::BendRouting => 1,
        }
    }
}

impl ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef> for EdgerouterStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef>> {
        match self {
            EdgerouterStrategy::DirectRouting => Box::new(StraightEdgeRouter::new()),
            EdgerouterStrategy::BendRouting => Box::new(BendEdgeRouter::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
