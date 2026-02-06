use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::p1treeify::DFSTreeifyer;
use crate::org::eclipse::elk::alg::mrtree::p2order::NodeOrderer;
use crate::org::eclipse::elk::alg::mrtree::p3place::NodePlacer;
use crate::org::eclipse::elk::alg::mrtree::p4route::EdgeRouter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum TreeLayoutPhases {
    P1Treeification,
    P2NodeOrdering,
    P3NodePlacement,
    P4EdgeRouting,
}

impl EnumSetType for TreeLayoutPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [TreeLayoutPhases; 4] = [
            TreeLayoutPhases::P1Treeification,
            TreeLayoutPhases::P2NodeOrdering,
            TreeLayoutPhases::P3NodePlacement,
            TreeLayoutPhases::P4EdgeRouting,
        ];
        &VARIANTS
    }
}

impl ILayoutPhaseFactory<TreeLayoutPhases, TGraphRef> for TreeLayoutPhases {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<TreeLayoutPhases, TGraphRef>> {
        match self {
            TreeLayoutPhases::P1Treeification => Box::new(DFSTreeifyer::default()),
            TreeLayoutPhases::P2NodeOrdering => Box::new(NodeOrderer::default()),
            TreeLayoutPhases::P3NodePlacement => Box::new(NodePlacer::default()),
            TreeLayoutPhases::P4EdgeRouting => Box::new(EdgeRouter),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(
            Self::variants()
                .iter()
                .position(|candidate| candidate == self)
                .unwrap_or(0),
        )
    }
}
