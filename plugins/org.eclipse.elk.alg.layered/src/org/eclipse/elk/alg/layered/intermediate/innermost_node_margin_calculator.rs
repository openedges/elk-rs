use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::node_dimension_calculation::NodeDimensionCalculation;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::transform::LGraphAdapters;
use crate::org::eclipse::elk::alg::layered::graph::LGraph;

pub struct InnermostNodeMarginCalculator;

impl ILayoutProcessor<LGraph> for InnermostNodeMarginCalculator {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Node margin calculation", 1.0);

        let adapter = LGraphAdapters::adapt(layered_graph, false, false, |_| true);
        NodeDimensionCalculation::get_node_margin_calculator(&adapter)
            .exclude_edge_head_tail_labels()
            .process();

        monitor.done();
    }
}
