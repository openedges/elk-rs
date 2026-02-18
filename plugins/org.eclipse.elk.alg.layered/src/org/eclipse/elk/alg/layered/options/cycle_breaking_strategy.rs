use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::p1cycles::{
    BfsNodeOrderCycleBreaker, DepthFirstCycleBreaker, DfsNodeOrderCycleBreaker, GreedyCycleBreaker,
    InteractiveCycleBreaker, ModelOrderCycleBreaker, ScConnectivityCycleBreaker,
    SccNodeTypeCycleBreaker,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CycleBreakingStrategy {
    #[default]
    Greedy,
    DepthFirst,
    Interactive,
    ModelOrder,
    GreedyModelOrder,
    SccConnectivity,
    SccNodeType,
    DfsNodeOrder,
    BfsNodeOrder,
}

impl CycleBreakingStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            CycleBreakingStrategy::Greedy => 0,
            CycleBreakingStrategy::DepthFirst => 1,
            CycleBreakingStrategy::Interactive => 2,
            CycleBreakingStrategy::ModelOrder => 3,
            CycleBreakingStrategy::GreedyModelOrder => 4,
            CycleBreakingStrategy::SccConnectivity => 5,
            CycleBreakingStrategy::SccNodeType => 6,
            CycleBreakingStrategy::DfsNodeOrder => 7,
            CycleBreakingStrategy::BfsNodeOrder => 8,
        }
    }
}

impl ILayoutPhaseFactory<LayeredPhases, LGraph> for CycleBreakingStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<LayeredPhases, LGraph>> {
        match self {
            CycleBreakingStrategy::Greedy => Box::new(GreedyCycleBreaker::new()),
            CycleBreakingStrategy::DepthFirst => Box::new(DepthFirstCycleBreaker::new()),
            CycleBreakingStrategy::ModelOrder => Box::new(ModelOrderCycleBreaker::new()),
            CycleBreakingStrategy::GreedyModelOrder => {
                Box::new(GreedyCycleBreaker::new_with_model_order(true))
            }
            CycleBreakingStrategy::SccConnectivity => Box::new(ScConnectivityCycleBreaker::new()),
            CycleBreakingStrategy::SccNodeType => Box::new(SccNodeTypeCycleBreaker::new()),
            CycleBreakingStrategy::DfsNodeOrder => Box::new(DfsNodeOrderCycleBreaker::new()),
            CycleBreakingStrategy::BfsNodeOrder => Box::new(BfsNodeOrderCycleBreaker::new()),
            CycleBreakingStrategy::Interactive => Box::new(InteractiveCycleBreaker::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
