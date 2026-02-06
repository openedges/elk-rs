use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use super::grid_elk_node::GridElkNode;
use super::i_node_arranger::INodeArranger;
use super::left_right_top_down_node_placer::LeftRightTopDownNodePlacer;
use super::topdown_packing_phases::TopdownPackingPhases;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NodeArrangementStrategy {
    #[default]
    LeftRightTopDownNodePlacer,
}

impl NodeArrangementStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            NodeArrangementStrategy::LeftRightTopDownNodePlacer => 0,
        }
    }

    pub fn create_arranger(&self) -> Box<dyn INodeArranger> {
        match self {
            NodeArrangementStrategy::LeftRightTopDownNodePlacer => {
                Box::new(LeftRightTopDownNodePlacer::new())
            }
        }
    }
}

impl ILayoutPhaseFactory<TopdownPackingPhases, GridElkNode> for NodeArrangementStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<TopdownPackingPhases, GridElkNode>> {
        match self {
            NodeArrangementStrategy::LeftRightTopDownNodePlacer => {
                Box::new(LeftRightTopDownNodePlacer::new())
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
