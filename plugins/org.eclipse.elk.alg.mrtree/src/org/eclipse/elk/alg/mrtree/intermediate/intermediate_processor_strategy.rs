use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

use crate::org::eclipse::elk::alg::mrtree::graph::TGraphRef;
use crate::org::eclipse::elk::alg::mrtree::intermediate::{
    compaction_processor::CompactionProcessor, direction_processor::DirectionProcessor,
    fan_processor::FanProcessor, graph_bounds_processor::GraphBoundsProcessor,
    level_coordinates_processor::LevelCoordinatesProcessor,
    level_height_processor::LevelHeightProcessor, level_processor::LevelProcessor,
    neighbors_processor::NeighborsProcessor, node_position_processor::NodePositionProcessor,
    root_processor::RootProcessor, untreeifyer::Untreeifyer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum IntermediateProcessorStrategy {
    RootProc,
    FanProc,
    LevelProc,
    NeighborsProc,
    LevelHeight,
    DirectionProc,
    NodePositionProc,
    CompactionProc,
    LevelCoords,
    GraphBoundsProc,
    DetreeifyingProc,
}

impl EnumSetType for IntermediateProcessorStrategy {
    fn variants() -> &'static [Self] {
        static VARIANTS: [IntermediateProcessorStrategy; 11] = [
            IntermediateProcessorStrategy::RootProc,
            IntermediateProcessorStrategy::FanProc,
            IntermediateProcessorStrategy::LevelProc,
            IntermediateProcessorStrategy::NeighborsProc,
            IntermediateProcessorStrategy::LevelHeight,
            IntermediateProcessorStrategy::DirectionProc,
            IntermediateProcessorStrategy::NodePositionProc,
            IntermediateProcessorStrategy::CompactionProc,
            IntermediateProcessorStrategy::LevelCoords,
            IntermediateProcessorStrategy::GraphBoundsProc,
            IntermediateProcessorStrategy::DetreeifyingProc,
        ];
        &VARIANTS
    }
}

impl ILayoutProcessorFactory<TGraphRef> for IntermediateProcessorStrategy {
    fn create(&self) -> Box<dyn ILayoutProcessor<TGraphRef>> {
        match self {
            IntermediateProcessorStrategy::RootProc => Box::new(RootProcessor::default()),
            IntermediateProcessorStrategy::FanProc => Box::new(FanProcessor::default()),
            IntermediateProcessorStrategy::LevelProc => Box::new(LevelProcessor::default()),
            IntermediateProcessorStrategy::NeighborsProc => Box::new(NeighborsProcessor::default()),
            IntermediateProcessorStrategy::LevelHeight => Box::new(LevelHeightProcessor::default()),
            IntermediateProcessorStrategy::DirectionProc => Box::new(DirectionProcessor::default()),
            IntermediateProcessorStrategy::NodePositionProc => {
                Box::new(NodePositionProcessor::default())
            }
            IntermediateProcessorStrategy::CompactionProc => Box::new(CompactionProcessor::default()),
            IntermediateProcessorStrategy::LevelCoords => Box::new(LevelCoordinatesProcessor::default()),
            IntermediateProcessorStrategy::GraphBoundsProc => Box::new(GraphBoundsProcessor::default()),
            IntermediateProcessorStrategy::DetreeifyingProc => Box::new(Untreeifyer::default()),
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
