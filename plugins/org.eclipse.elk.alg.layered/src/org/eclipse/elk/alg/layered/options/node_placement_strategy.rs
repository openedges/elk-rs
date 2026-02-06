use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::p4nodes::{
    BKNodePlacer, InteractiveNodePlacer, LinearSegmentsNodePlacer, NetworkSimplexPlacer,
    SimpleNodePlacer,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum NodePlacementStrategy {
    Simple,
    Interactive,
    LinearSegments,
    #[default]
    BrandesKoepf,
    NetworkSimplex,
}

impl NodePlacementStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            NodePlacementStrategy::Simple => 0,
            NodePlacementStrategy::Interactive => 1,
            NodePlacementStrategy::LinearSegments => 2,
            NodePlacementStrategy::BrandesKoepf => 3,
            NodePlacementStrategy::NetworkSimplex => 4,
        }
    }
}

impl ILayoutPhaseFactory<LayeredPhases, LGraph> for NodePlacementStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<LayeredPhases, LGraph>> {
        match self {
            NodePlacementStrategy::Simple => Box::new(SimpleNodePlacer::new()),
            NodePlacementStrategy::Interactive => Box::new(InteractiveNodePlacer::new()),
            NodePlacementStrategy::LinearSegments => Box::new(LinearSegmentsNodePlacer::new()),
            NodePlacementStrategy::BrandesKoepf => Box::new(BKNodePlacer::new()),
            NodePlacementStrategy::NetworkSimplex => Box::new(NetworkSimplexPlacer::new()),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
