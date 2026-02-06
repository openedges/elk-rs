use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::radial::intermediate::calculate_graph_size::CalculateGraphSize;
use crate::org::eclipse::elk::alg::radial::intermediate::compaction::GeneralCompactor;
use crate::org::eclipse::elk::alg::radial::intermediate::edge_angle_calculator::EdgeAngleCalculator;
use crate::org::eclipse::elk::alg::radial::intermediate::overlaps::RadiusExtensionOverlapRemoval;
use crate::org::eclipse::elk::alg::radial::intermediate::rotation::GeneralRotator;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum IntermediateProcessorStrategy {
    OverlapRemoval,
    Compaction,
    Rotation,
    GraphSizeCalculation,
    OutgoingEdgeAngles,
}

impl EnumSetType for IntermediateProcessorStrategy {
    fn variants() -> &'static [Self] {
        static VARIANTS: [IntermediateProcessorStrategy; 5] = [
            IntermediateProcessorStrategy::OverlapRemoval,
            IntermediateProcessorStrategy::Compaction,
            IntermediateProcessorStrategy::Rotation,
            IntermediateProcessorStrategy::GraphSizeCalculation,
            IntermediateProcessorStrategy::OutgoingEdgeAngles,
        ];
        &VARIANTS
    }
}

impl ILayoutProcessorFactory<ElkNodeRef> for IntermediateProcessorStrategy {
    fn create(&self) -> Box<dyn ILayoutProcessor<ElkNodeRef>> {
        match self {
            IntermediateProcessorStrategy::OverlapRemoval => {
                Box::new(RadiusExtensionOverlapRemoval::default())
            }
            IntermediateProcessorStrategy::Compaction => Box::new(GeneralCompactor),
            IntermediateProcessorStrategy::Rotation => Box::new(GeneralRotator),
            IntermediateProcessorStrategy::GraphSizeCalculation => Box::new(CalculateGraphSize),
            IntermediateProcessorStrategy::OutgoingEdgeAngles => Box::new(EdgeAngleCalculator),
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
