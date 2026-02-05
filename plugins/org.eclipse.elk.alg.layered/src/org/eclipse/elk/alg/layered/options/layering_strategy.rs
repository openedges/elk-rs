use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::p2layers::{
    BreadthFirstModelOrderLayerer, CoffmanGrahamLayerer, DepthFirstModelOrderLayerer,
    InteractiveLayerer, LongestPathLayerer, LongestPathSourceLayerer, MinWidthLayerer,
    NetworkSimplexLayerer, StretchWidthLayerer,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum LayeringStrategy {
    NetworkSimplex,
    LongestPath,
    LongestPathSource,
    CoffmanGraham,
    Interactive,
    StretchWidth,
    MinWidth,
    BfModelOrder,
    DfModelOrder,
}

impl LayeringStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            LayeringStrategy::NetworkSimplex => 0,
            LayeringStrategy::LongestPath => 1,
            LayeringStrategy::LongestPathSource => 2,
            LayeringStrategy::CoffmanGraham => 3,
            LayeringStrategy::Interactive => 4,
            LayeringStrategy::StretchWidth => 5,
            LayeringStrategy::MinWidth => 6,
            LayeringStrategy::BfModelOrder => 7,
            LayeringStrategy::DfModelOrder => 8,
        }
    }
}

impl Default for LayeringStrategy {
    fn default() -> Self {
        LayeringStrategy::NetworkSimplex
    }
}

impl ILayoutPhaseFactory<LayeredPhases, LGraph> for LayeringStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<LayeredPhases, LGraph>> {
        match self {
            LayeringStrategy::NetworkSimplex => Box::new(NetworkSimplexLayerer::new()),
            LayeringStrategy::LongestPath => Box::new(LongestPathLayerer::new()),
            LayeringStrategy::LongestPathSource => Box::new(LongestPathSourceLayerer::new()),
            LayeringStrategy::CoffmanGraham => Box::new(CoffmanGrahamLayerer::new()),
            LayeringStrategy::Interactive => Box::new(InteractiveLayerer::new()),
            LayeringStrategy::StretchWidth => Box::new(StretchWidthLayerer::new()),
            LayeringStrategy::MinWidth => Box::new(MinWidthLayerer::new()),
            LayeringStrategy::BfModelOrder => Box::new(BreadthFirstModelOrderLayerer::new()),
            LayeringStrategy::DfModelOrder => Box::new(DepthFirstModelOrderLayerer::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
