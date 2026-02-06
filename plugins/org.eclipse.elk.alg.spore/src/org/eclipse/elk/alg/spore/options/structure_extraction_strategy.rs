use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;

use crate::org::eclipse::elk::alg::spore::graph::Graph;
use crate::org::eclipse::elk::alg::spore::p1structure::DelaunayTriangulationPhase;
use crate::org::eclipse::elk::alg::spore::spore_phases::SPOrEPhases;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum StructureExtractionStrategy {
    #[default]
    DelaunayTriangulation,
}

impl StructureExtractionStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            StructureExtractionStrategy::DelaunayTriangulation => 0,
        }
    }
}

impl ILayoutPhaseFactory<SPOrEPhases, Graph> for StructureExtractionStrategy {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<SPOrEPhases, Graph>> {
        match self {
            StructureExtractionStrategy::DelaunayTriangulation => {
                Box::new(DelaunayTriangulationPhase::new())
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
