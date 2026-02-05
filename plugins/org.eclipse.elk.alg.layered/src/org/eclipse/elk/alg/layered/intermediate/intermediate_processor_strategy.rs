use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSetType, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::intermediate::final_spline_bendpoints_calculator::FinalSplineBendpointsCalculator;
use crate::org::eclipse::elk::alg::layered::intermediate::graph_transformer::{
    GraphTransformer, Mode as GraphTransformerMode,
};
use crate::org::eclipse::elk::alg::layered::intermediate::{
    LabelAndNodeSizeProcessor, NorthSouthPortPostprocessor, NorthSouthPortPreprocessor,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum IntermediateProcessorStrategy {
    DirectionPreprocessor,
    CommentPreprocessor,
    EdgeAndLayerConstraintEdgeReverser,
    InteractiveExternalPortPositioner,
    PartitionPreprocessor,
    LabelDummyInserter,
    SelfLoopPreprocessor,
    LayerConstraintPreprocessor,
    PartitionMidprocessor,
    HighDegreeNodeLayerProcessor,
    NodePromotion,
    LayerConstraintPostprocessor,
    PartitionPostprocessor,
    HierarchicalPortConstraintProcessor,
    SemiInteractiveCrossminProcessor,
    BreakingPointInserter,
    LongEdgeSplitter,
    PortSideProcessor,
    InvertedPortProcessor,
    PortListSorter,
    SortByInputOrderOfModel,
    NorthSouthPortPreprocessor,
    BreakingPointProcessor,
    OneSidedGreedySwitch,
    TwoSidedGreedySwitch,
    SelfLoopPortRestorer,
    AlternatingLayerUnzipper,
    SingleEdgeGraphWrapper,
    InLayerConstraintProcessor,
    EndNodePortLabelManagementProcessor,
    LabelAndNodeSizeProcessor,
    InnermostNodeMarginCalculator,
    SelfLoopRouter,
    CommentNodeMarginCalculator,
    EndLabelPreprocessor,
    LabelDummySwitcher,
    CenterLabelManagementProcessor,
    LabelSideSelector,
    HyperedgeDummyMerger,
    HierarchicalPortDummySizeProcessor,
    LayerSizeAndGraphHeightCalculator,
    HierarchicalPortPositionProcessor,
    ConstraintsPostprocessor,
    CommentPostprocessor,
    HypernodeProcessor,
    HierarchicalPortOrthogonalEdgeRouter,
    LongEdgeJoiner,
    SelfLoopPostprocessor,
    BreakingPointRemover,
    NorthSouthPortPostprocessor,
    HorizontalCompactor,
    LabelDummyRemover,
    FinalSplineBendpointsCalculator,
    EndLabelSorter,
    ReversedEdgeRestorer,
    EndLabelPostprocessor,
    HierarchicalNodeResizer,
    DirectionPostprocessor,
}

impl EnumSetType for IntermediateProcessorStrategy {
    fn variants() -> &'static [Self] {
        static VARIANTS: [IntermediateProcessorStrategy; 58] = [
            IntermediateProcessorStrategy::DirectionPreprocessor,
            IntermediateProcessorStrategy::CommentPreprocessor,
            IntermediateProcessorStrategy::EdgeAndLayerConstraintEdgeReverser,
            IntermediateProcessorStrategy::InteractiveExternalPortPositioner,
            IntermediateProcessorStrategy::PartitionPreprocessor,
            IntermediateProcessorStrategy::LabelDummyInserter,
            IntermediateProcessorStrategy::SelfLoopPreprocessor,
            IntermediateProcessorStrategy::LayerConstraintPreprocessor,
            IntermediateProcessorStrategy::PartitionMidprocessor,
            IntermediateProcessorStrategy::HighDegreeNodeLayerProcessor,
            IntermediateProcessorStrategy::NodePromotion,
            IntermediateProcessorStrategy::LayerConstraintPostprocessor,
            IntermediateProcessorStrategy::PartitionPostprocessor,
            IntermediateProcessorStrategy::HierarchicalPortConstraintProcessor,
            IntermediateProcessorStrategy::SemiInteractiveCrossminProcessor,
            IntermediateProcessorStrategy::BreakingPointInserter,
            IntermediateProcessorStrategy::LongEdgeSplitter,
            IntermediateProcessorStrategy::PortSideProcessor,
            IntermediateProcessorStrategy::InvertedPortProcessor,
            IntermediateProcessorStrategy::PortListSorter,
            IntermediateProcessorStrategy::SortByInputOrderOfModel,
            IntermediateProcessorStrategy::NorthSouthPortPreprocessor,
            IntermediateProcessorStrategy::BreakingPointProcessor,
            IntermediateProcessorStrategy::OneSidedGreedySwitch,
            IntermediateProcessorStrategy::TwoSidedGreedySwitch,
            IntermediateProcessorStrategy::SelfLoopPortRestorer,
            IntermediateProcessorStrategy::AlternatingLayerUnzipper,
            IntermediateProcessorStrategy::SingleEdgeGraphWrapper,
            IntermediateProcessorStrategy::InLayerConstraintProcessor,
            IntermediateProcessorStrategy::EndNodePortLabelManagementProcessor,
            IntermediateProcessorStrategy::LabelAndNodeSizeProcessor,
            IntermediateProcessorStrategy::InnermostNodeMarginCalculator,
            IntermediateProcessorStrategy::SelfLoopRouter,
            IntermediateProcessorStrategy::CommentNodeMarginCalculator,
            IntermediateProcessorStrategy::EndLabelPreprocessor,
            IntermediateProcessorStrategy::LabelDummySwitcher,
            IntermediateProcessorStrategy::CenterLabelManagementProcessor,
            IntermediateProcessorStrategy::LabelSideSelector,
            IntermediateProcessorStrategy::HyperedgeDummyMerger,
            IntermediateProcessorStrategy::HierarchicalPortDummySizeProcessor,
            IntermediateProcessorStrategy::LayerSizeAndGraphHeightCalculator,
            IntermediateProcessorStrategy::HierarchicalPortPositionProcessor,
            IntermediateProcessorStrategy::ConstraintsPostprocessor,
            IntermediateProcessorStrategy::CommentPostprocessor,
            IntermediateProcessorStrategy::HypernodeProcessor,
            IntermediateProcessorStrategy::HierarchicalPortOrthogonalEdgeRouter,
            IntermediateProcessorStrategy::LongEdgeJoiner,
            IntermediateProcessorStrategy::SelfLoopPostprocessor,
            IntermediateProcessorStrategy::BreakingPointRemover,
            IntermediateProcessorStrategy::NorthSouthPortPostprocessor,
            IntermediateProcessorStrategy::HorizontalCompactor,
            IntermediateProcessorStrategy::LabelDummyRemover,
            IntermediateProcessorStrategy::FinalSplineBendpointsCalculator,
            IntermediateProcessorStrategy::EndLabelSorter,
            IntermediateProcessorStrategy::ReversedEdgeRestorer,
            IntermediateProcessorStrategy::EndLabelPostprocessor,
            IntermediateProcessorStrategy::HierarchicalNodeResizer,
            IntermediateProcessorStrategy::DirectionPostprocessor,
        ];
        &VARIANTS
    }
}

impl ILayoutProcessorFactory<LGraph> for IntermediateProcessorStrategy {
    fn create(&self) -> Box<dyn ILayoutProcessor<LGraph>> {
        match self {
            IntermediateProcessorStrategy::DirectionPostprocessor => Box::new(GraphTransformer::new(
                GraphTransformerMode::ToInternalLtr,
            )),
            IntermediateProcessorStrategy::DirectionPreprocessor => Box::new(GraphTransformer::new(
                GraphTransformerMode::ToInputDirection,
            )),
            IntermediateProcessorStrategy::NorthSouthPortPreprocessor => {
                Box::new(NorthSouthPortPreprocessor::default())
            }
            IntermediateProcessorStrategy::NorthSouthPortPostprocessor => {
                Box::new(NorthSouthPortPostprocessor::default())
            }
            IntermediateProcessorStrategy::FinalSplineBendpointsCalculator => {
                Box::new(FinalSplineBendpointsCalculator::default())
            }
            IntermediateProcessorStrategy::LabelAndNodeSizeProcessor => {
                Box::new(LabelAndNodeSizeProcessor::default())
            }
            _ => Box::new(NoOpLayoutProcessor),
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

struct NoOpLayoutProcessor;

impl ILayoutProcessor<LGraph> for NoOpLayoutProcessor {
    fn process(&mut self, _graph: &mut LGraph, _progress_monitor: &mut dyn IElkProgressMonitor) {}
}
