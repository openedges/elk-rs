use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::no_op_phase::NoOpPhase;
use crate::org::eclipse::elk::alg::layered::p3order::{
    CrossMinType, LayerSweepCrossingMinimizer, NoCrossingMinimizer,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CrossingMinimizationStrategy {
    LayerSweep,
    MedianLayerSweep,
    Interactive,
    None,
}

impl CrossingMinimizationStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            CrossingMinimizationStrategy::LayerSweep => 0,
            CrossingMinimizationStrategy::MedianLayerSweep => 1,
            CrossingMinimizationStrategy::Interactive => 2,
            CrossingMinimizationStrategy::None => 3,
        }
    }
}

impl Default for CrossingMinimizationStrategy {
    fn default() -> Self {
        CrossingMinimizationStrategy::LayerSweep
    }
}

impl ILayoutPhaseFactory<LayeredPhases, LGraph> for CrossingMinimizationStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<LayeredPhases, LGraph>> {
        match self {
            CrossingMinimizationStrategy::None => Box::new(NoCrossingMinimizer::new()),
            CrossingMinimizationStrategy::LayerSweep => {
                Box::new(LayerSweepCrossingMinimizer::new(CrossMinType::Barycenter))
            }
            CrossingMinimizationStrategy::MedianLayerSweep => {
                Box::new(LayerSweepCrossingMinimizer::new(CrossMinType::Median))
            }
            _ => Box::new(NoOpPhase::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
