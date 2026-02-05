use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    ElkGraphAdapter, ElkGraphAdapters,
};
use org_eclipse_elk_graph::org::eclipse::elk::graph::ElkNodeRef;

use crate::org::eclipse::elk::alg::common::nodespacing::NodeDimensionCalculation;

pub struct NodeMicroLayout {
    adapter: ElkGraphAdapter,
}

impl NodeMicroLayout {
    pub fn for_graph(graph: ElkNodeRef) -> Self {
        NodeMicroLayout {
            adapter: ElkGraphAdapters::adapt(graph),
        }
    }

    pub fn for_adapter(adapter: ElkGraphAdapter) -> Self {
        NodeMicroLayout { adapter }
    }

    pub fn execute(&self) {
        NodeDimensionCalculation::sort_port_lists(&self.adapter);
        NodeDimensionCalculation::calculate_label_and_node_sizes(&self.adapter);
        NodeDimensionCalculation::calculate_node_margins(&self.adapter);
    }
}
