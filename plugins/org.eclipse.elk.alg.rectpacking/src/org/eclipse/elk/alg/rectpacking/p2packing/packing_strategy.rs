use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;

use super::{Compactor, NoPlacement, SimplePlacement};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PackingStrategy {
    Compaction,
    Simple,
    None,
}

impl Default for PackingStrategy {
    fn default() -> Self {
        PackingStrategy::Compaction
    }
}

impl PackingStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            PackingStrategy::Compaction => 0,
            PackingStrategy::Simple => 1,
            PackingStrategy::None => 2,
        }
    }
}

impl ILayoutPhaseFactory<RectPackingLayoutPhases, ElkNodeRef> for PackingStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef>> {
        match self {
            PackingStrategy::Compaction => Box::new(Compactor::new()),
            PackingStrategy::Simple => Box::new(SimplePlacement::new()),
            PackingStrategy::None => Box::new(NoPlacement::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
