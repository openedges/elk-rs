use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSetType, IElkProgressMonitor};

use crate::org::eclipse::elk::alg::layered::graph::LGraph;
use crate::org::eclipse::elk::alg::layered::intermediate::final_spline_bendpoints_calculator::FinalSplineBendpointsCalculator;
use crate::org::eclipse::elk::alg::layered::intermediate::graph_transformer::{
    GraphTransformer, Mode as GraphTransformerMode,
};
use crate::org::eclipse::elk::alg::layered::intermediate::{
    AlternatingLayerUnzipper, BreakingPointInserter, BreakingPointProcessor, BreakingPointRemover,
    CommentNodeMarginCalculator, CommentPostprocessor, CommentPreprocessor,
    ConstraintsPostprocessor, EdgeAndLayerConstraintEdgeReverser, EndLabelPostprocessor,
    EndLabelPreprocessor, EndLabelSorter, HierarchicalNodeResizingProcessor,
    HierarchicalPortConstraintProcessor, HierarchicalPortDummySizeProcessor,
    HierarchicalPortOrthogonalEdgeRouter, HierarchicalPortPositionProcessor,
    HighDegreeNodeLayeringProcessor, HorizontalGraphCompactor, HyperedgeDummyMerger,
    HypernodeProcessor, InLayerConstraintProcessor, InnermostNodeMarginCalculator,
    InteractiveExternalPortPositioner, InvertedPortProcessor, LabelAndNodeSizeProcessor,
    LabelDummyInserter, LabelDummyRemover, LabelDummySwitcher, LabelManagementProcessor,
    LabelSideSelector, LayerConstraintPostprocessor, LayerConstraintPreprocessor,
    LayerSizeAndGraphHeightCalculator, LongEdgeJoiner, LongEdgeSplitter, NodePromotion,
    NorthSouthPortPostprocessor, NorthSouthPortPreprocessor, PartitionMidprocessor,
    PartitionPostprocessor, PartitionPreprocessor, PortListSorter, PortSideProcessor,
    ReversedEdgeRestorer, SelfLoopPortRestorer, SelfLoopPostProcessor, SelfLoopPreProcessor,
    SelfLoopRouter, SemiInteractiveCrossMinProcessor, SingleEdgeGraphWrapper,
    SortByInputModelProcessor,
};
use crate::org::eclipse::elk::alg::layered::p3order::{CrossMinType, LayerSweepCrossingMinimizer};

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
            IntermediateProcessorStrategy::DirectionPostprocessor => {
                Box::new(GraphTransformer::new(GraphTransformerMode::ToInternalLtr))
            }
            IntermediateProcessorStrategy::DirectionPreprocessor => Box::new(
                GraphTransformer::new(GraphTransformerMode::ToInputDirection),
            ),
            IntermediateProcessorStrategy::NorthSouthPortPreprocessor => {
                Box::new(NorthSouthPortPreprocessor)
            }
            IntermediateProcessorStrategy::NorthSouthPortPostprocessor => {
                Box::new(NorthSouthPortPostprocessor)
            }
            IntermediateProcessorStrategy::FinalSplineBendpointsCalculator => {
                Box::new(FinalSplineBendpointsCalculator::default())
            }
            IntermediateProcessorStrategy::LabelAndNodeSizeProcessor => {
                Box::new(LabelAndNodeSizeProcessor)
            }
            IntermediateProcessorStrategy::LabelDummyInserter => Box::new(LabelDummyInserter),
            IntermediateProcessorStrategy::PortListSorter => Box::new(PortListSorter),
            IntermediateProcessorStrategy::SortByInputOrderOfModel => {
                Box::new(SortByInputModelProcessor)
            }
            IntermediateProcessorStrategy::EdgeAndLayerConstraintEdgeReverser => {
                Box::new(EdgeAndLayerConstraintEdgeReverser)
            }
            IntermediateProcessorStrategy::LayerConstraintPreprocessor => {
                Box::new(LayerConstraintPreprocessor)
            }
            IntermediateProcessorStrategy::LayerConstraintPostprocessor => {
                Box::new(LayerConstraintPostprocessor)
            }
            IntermediateProcessorStrategy::PartitionPreprocessor => Box::new(PartitionPreprocessor),
            IntermediateProcessorStrategy::PartitionMidprocessor => Box::new(PartitionMidprocessor),
            IntermediateProcessorStrategy::PartitionPostprocessor => {
                Box::new(PartitionPostprocessor)
            }
            IntermediateProcessorStrategy::HighDegreeNodeLayerProcessor => {
                Box::new(HighDegreeNodeLayeringProcessor::default())
            }
            IntermediateProcessorStrategy::SelfLoopPreprocessor => Box::new(SelfLoopPreProcessor),
            IntermediateProcessorStrategy::SelfLoopPortRestorer => Box::new(SelfLoopPortRestorer),
            IntermediateProcessorStrategy::AlternatingLayerUnzipper => {
                Box::new(AlternatingLayerUnzipper)
            }
            IntermediateProcessorStrategy::SingleEdgeGraphWrapper => {
                Box::new(SingleEdgeGraphWrapper)
            }
            IntermediateProcessorStrategy::InLayerConstraintProcessor => {
                Box::new(InLayerConstraintProcessor)
            }
            IntermediateProcessorStrategy::InvertedPortProcessor => Box::new(InvertedPortProcessor),
            IntermediateProcessorStrategy::PortSideProcessor => Box::new(PortSideProcessor),
            IntermediateProcessorStrategy::NodePromotion => Box::new(NodePromotion),
            IntermediateProcessorStrategy::SemiInteractiveCrossminProcessor => {
                Box::new(SemiInteractiveCrossMinProcessor)
            }
            IntermediateProcessorStrategy::OneSidedGreedySwitch => Box::new(
                LayerSweepAsProcessor::new(CrossMinType::OneSidedGreedySwitch),
            ),
            IntermediateProcessorStrategy::TwoSidedGreedySwitch => Box::new(
                LayerSweepAsProcessor::new(CrossMinType::TwoSidedGreedySwitch),
            ),
            IntermediateProcessorStrategy::LongEdgeSplitter => Box::new(LongEdgeSplitter),
            IntermediateProcessorStrategy::LongEdgeJoiner => Box::new(LongEdgeJoiner),
            IntermediateProcessorStrategy::BreakingPointInserter => Box::new(BreakingPointInserter),
            IntermediateProcessorStrategy::BreakingPointProcessor => {
                Box::new(BreakingPointProcessor)
            }
            IntermediateProcessorStrategy::BreakingPointRemover => Box::new(BreakingPointRemover),
            IntermediateProcessorStrategy::ReversedEdgeRestorer => Box::new(ReversedEdgeRestorer),
            IntermediateProcessorStrategy::LayerSizeAndGraphHeightCalculator => {
                Box::new(LayerSizeAndGraphHeightCalculator)
            }
            IntermediateProcessorStrategy::CommentNodeMarginCalculator => {
                Box::new(CommentNodeMarginCalculator)
            }
            IntermediateProcessorStrategy::InnermostNodeMarginCalculator => {
                Box::new(InnermostNodeMarginCalculator)
            }
            IntermediateProcessorStrategy::LabelSideSelector => Box::new(LabelSideSelector),
            IntermediateProcessorStrategy::HyperedgeDummyMerger => Box::new(HyperedgeDummyMerger),
            IntermediateProcessorStrategy::LabelDummySwitcher => {
                Box::new(LabelDummySwitcher::default())
            }
            IntermediateProcessorStrategy::LabelDummyRemover => Box::new(LabelDummyRemover),
            IntermediateProcessorStrategy::EndLabelSorter => Box::new(EndLabelSorter),
            IntermediateProcessorStrategy::EndLabelPreprocessor => Box::new(EndLabelPreprocessor),
            IntermediateProcessorStrategy::EndLabelPostprocessor => Box::new(EndLabelPostprocessor),
            IntermediateProcessorStrategy::CommentPostprocessor => Box::new(CommentPostprocessor),
            IntermediateProcessorStrategy::CommentPreprocessor => Box::new(CommentPreprocessor),
            IntermediateProcessorStrategy::SelfLoopPostprocessor => Box::new(SelfLoopPostProcessor),
            IntermediateProcessorStrategy::SelfLoopRouter => Box::new(SelfLoopRouter),
            IntermediateProcessorStrategy::InteractiveExternalPortPositioner => {
                Box::new(InteractiveExternalPortPositioner)
            }
            IntermediateProcessorStrategy::HierarchicalNodeResizer => {
                Box::new(HierarchicalNodeResizingProcessor)
            }
            IntermediateProcessorStrategy::HierarchicalPortConstraintProcessor => {
                Box::new(HierarchicalPortConstraintProcessor)
            }
            IntermediateProcessorStrategy::HierarchicalPortDummySizeProcessor => {
                Box::new(HierarchicalPortDummySizeProcessor)
            }
            IntermediateProcessorStrategy::HierarchicalPortPositionProcessor => {
                Box::new(HierarchicalPortPositionProcessor)
            }
            IntermediateProcessorStrategy::HierarchicalPortOrthogonalEdgeRouter => {
                Box::new(HierarchicalPortOrthogonalEdgeRouter::default())
            }
            IntermediateProcessorStrategy::ConstraintsPostprocessor => {
                Box::new(ConstraintsPostprocessor)
            }
            IntermediateProcessorStrategy::HypernodeProcessor => Box::new(HypernodeProcessor),
            IntermediateProcessorStrategy::EndNodePortLabelManagementProcessor => {
                Box::new(LabelManagementProcessor::new(false))
            }
            IntermediateProcessorStrategy::CenterLabelManagementProcessor => {
                Box::new(LabelManagementProcessor::new(true))
            }
            IntermediateProcessorStrategy::HorizontalCompactor => {
                Box::new(HorizontalGraphCompactor)
            }
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

struct LayerSweepAsProcessor {
    minimizer: LayerSweepCrossingMinimizer,
}

impl LayerSweepAsProcessor {
    fn new(cross_min_type: CrossMinType) -> Self {
        Self {
            minimizer: LayerSweepCrossingMinimizer::new(cross_min_type),
        }
    }
}

impl ILayoutProcessor<LGraph> for LayerSweepAsProcessor {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        ILayoutPhase::process(&mut self.minimizer, graph, progress_monitor);
    }
}
