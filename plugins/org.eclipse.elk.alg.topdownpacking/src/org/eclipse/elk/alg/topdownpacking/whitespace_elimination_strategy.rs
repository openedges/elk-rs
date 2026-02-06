use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use super::bottom_row_equal_whitespace_eliminator::BottomRowEqualWhitespaceEliminator;
use super::grid_elk_node::GridElkNode;
use super::topdown_packing_phases::TopdownPackingPhases;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum WhitespaceEliminationStrategy {
    #[default]
    BottomRowEqualWhitespaceEliminator,
}

impl WhitespaceEliminationStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            WhitespaceEliminationStrategy::BottomRowEqualWhitespaceEliminator => 0,
        }
    }
}

impl ILayoutPhaseFactory<TopdownPackingPhases, GridElkNode> for WhitespaceEliminationStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<TopdownPackingPhases, GridElkNode>> {
        match self {
            WhitespaceEliminationStrategy::BottomRowEqualWhitespaceEliminator => {
                Box::new(BottomRowEqualWhitespaceEliminator::new())
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
