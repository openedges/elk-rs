use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::p1position::EadesRadial;
use crate::org::eclipse::elk::alg::radial::p2routing::StraightLineEdgeRouter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum RadialLayoutPhases {
    P1NodePlacement,
    P2EdgeRouting,
}

impl EnumSetType for RadialLayoutPhases {
    fn variants() -> &'static [Self] {
        static VARIANTS: [RadialLayoutPhases; 2] = [
            RadialLayoutPhases::P1NodePlacement,
            RadialLayoutPhases::P2EdgeRouting,
        ];
        &VARIANTS
    }
}

impl ILayoutPhaseFactory<RadialLayoutPhases, ElkNodeRef> for RadialLayoutPhases {
    fn create_phase(&self) -> Box<dyn ILayoutPhase<RadialLayoutPhases, ElkNodeRef>> {
        match self {
            RadialLayoutPhases::P1NodePlacement => Box::new(EadesRadial::new()),
            RadialLayoutPhases::P2EdgeRouting => Box::new(StraightLineEdgeRouter::new()),
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
