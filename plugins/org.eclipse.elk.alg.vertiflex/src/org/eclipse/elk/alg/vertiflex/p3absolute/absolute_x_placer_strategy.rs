use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::p3absolute::AbsoluteXPlacer;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum AbsoluteXPlacerStrategy {
    #[default]
    AbsoluteXPlacing,
}

impl AbsoluteXPlacerStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            AbsoluteXPlacerStrategy::AbsoluteXPlacing => 0,
        }
    }
}

impl ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef> for AbsoluteXPlacerStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef>> {
        match self {
            AbsoluteXPlacerStrategy::AbsoluteXPlacing => Box::new(AbsoluteXPlacer::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
