use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::vertiflex::p2relative::RelativeXPlacer;
use crate::org::eclipse::elk::alg::vertiflex::vertiflex_layout_phases::VertiFlexLayoutPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum RelativeXPlacerStrategy {
    #[default]
    SimpleXPlacing,
}

impl RelativeXPlacerStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            RelativeXPlacerStrategy::SimpleXPlacing => 0,
        }
    }
}

impl ILayoutPhaseFactory<VertiFlexLayoutPhases, ElkNodeRef> for RelativeXPlacerStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<VertiFlexLayoutPhases, ElkNodeRef>> {
        match self {
            RelativeXPlacerStrategy::SimpleXPlacing => Box::new(RelativeXPlacer::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
