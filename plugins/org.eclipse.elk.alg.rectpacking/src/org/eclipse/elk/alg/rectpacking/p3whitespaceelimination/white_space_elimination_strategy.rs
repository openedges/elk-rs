use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::rectpacking::rect_packing_layout_phases::RectPackingLayoutPhases;

use super::{EqualWhitespaceEliminator, ToAspectratioNodeExpander};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
pub enum WhiteSpaceEliminationStrategy {
    EqualBetweenStructures,
    ToAspectRatio,
    #[default]
    None,
}

impl WhiteSpaceEliminationStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            WhiteSpaceEliminationStrategy::EqualBetweenStructures => 0,
            WhiteSpaceEliminationStrategy::ToAspectRatio => 1,
            WhiteSpaceEliminationStrategy::None => 2,
        }
    }
}

impl ILayoutPhaseFactory<RectPackingLayoutPhases, ElkNodeRef> for WhiteSpaceEliminationStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<RectPackingLayoutPhases, ElkNodeRef>> {
        match self {
            WhiteSpaceEliminationStrategy::EqualBetweenStructures => {
                Box::new(EqualWhitespaceEliminator::new())
            }
            WhiteSpaceEliminationStrategy::ToAspectRatio => {
                Box::new(ToAspectratioNodeExpander::new())
            }
            WhiteSpaceEliminationStrategy::None => Box::new(EqualWhitespaceEliminator::noop()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
