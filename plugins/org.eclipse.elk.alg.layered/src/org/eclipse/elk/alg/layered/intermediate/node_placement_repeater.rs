use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::nodespacing::{
    node_dimension_calculation::NodeDimensionCalculation,
    node_label_and_size_calculator::NodeLabelAndSizeCalculator,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::{
    i_layout_phase_factory::ILayoutPhaseFactory, i_layout_processor::ILayoutProcessor,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::adapters::{
    GraphAdapter, GraphElementAdapter,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::transform::LGraphAdapters;
use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

/// Repeats node placement with the sizes produced by node flexibility fixed.
pub struct NodePlacementRepeater;

impl ILayoutProcessor<LGraph> for NodePlacementRepeater {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Node placement repeater", 1.0);

        let strategy = graph.get_property(
            LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_RECOMPUTE_NODE_PLACEMENT,
        ).expect("Missing second node placement strategy");
        let mut node_placer = strategy.create_phase();
        graph.set_property(
            LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY_DEFAULT,
            None,
        );
        for layer in graph.layers() {
            for node in layer.lock().nodes() {
                let mut node = node.lock();
                node.set_property(
                    LayeredOptions::NODE_PLACEMENT_NETWORK_SIMPLEX_NODE_FLEXIBILITY,
                    None,
                );
                // Removing the constraints restores their fixed-size default.
                node.set_property(LayeredOptions::NODE_SIZE_CONSTRAINTS, None);
                node.shape().position().y = 0.0;
            }
        }

        let adapter = LGraphAdapters::adapt(graph, true, true, |node| {
            node.node_type() == NodeType::Normal
        });
        let direction = adapter
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        // Use the full calculator directly: its fixed-size path repositions ports
        // and labels without growing the sizes chosen by the first placer.
        for node in adapter.get_nodes() {
            NodeLabelAndSizeCalculator::process(&node, direction);
        }
        NodeDimensionCalculation::calculate_node_margins(&adapter);

        // Placers build fresh arena snapshots from the modified graph. No graph
        // mutex is reacquired while the surrounding layout owns its guard.
        node_placer.process(graph, monitor);
        monitor.done();
    }
}
