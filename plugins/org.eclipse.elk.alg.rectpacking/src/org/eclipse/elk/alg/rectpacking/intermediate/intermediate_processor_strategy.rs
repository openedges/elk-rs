use std::any::Any;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use super::{
    InteractiveNodeReorderer, MinSizePostProcessor, MinSizePreProcessor, NodeSizeReorderer,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum IntermediateProcessorStrategy {
    NodeSizeReorderer,
    InteractiveNodeReorderer,
    MinSizePreProcessor,
    MinSizePostProcessor,
}

impl IntermediateProcessorStrategy {
    pub fn ordinal(self) -> usize {
        match self {
            IntermediateProcessorStrategy::NodeSizeReorderer => 0,
            IntermediateProcessorStrategy::InteractiveNodeReorderer => 1,
            IntermediateProcessorStrategy::MinSizePreProcessor => 2,
            IntermediateProcessorStrategy::MinSizePostProcessor => 3,
        }
    }
}

impl ILayoutProcessorFactory<ElkNodeRef> for IntermediateProcessorStrategy {
    fn create(&self) -> Box<dyn ILayoutProcessor<ElkNodeRef>> {
        match self {
            IntermediateProcessorStrategy::NodeSizeReorderer => Box::new(NodeSizeReorderer),
            IntermediateProcessorStrategy::InteractiveNodeReorderer => {
                Box::new(InteractiveNodeReorderer)
            }
            IntermediateProcessorStrategy::MinSizePreProcessor => Box::new(MinSizePreProcessor),
            IntermediateProcessorStrategy::MinSizePostProcessor => Box::new(MinSizePostProcessor),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn enum_ordinal(&self) -> Option<usize> {
        Some(self.ordinal())
    }
}
