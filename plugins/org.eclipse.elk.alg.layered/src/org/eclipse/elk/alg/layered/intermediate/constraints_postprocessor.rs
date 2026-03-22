use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct ConstraintsPostprocessor;

impl ILayoutProcessor<LGraph> for ConstraintsPostprocessor {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Constraints Postprocessor", 1.0);

        let mut layer_index: i32 = 0;
        for layer in graph.layers().clone() {
            let nodes = layer
                .lock().nodes().clone();

            let mut position_index: i32 = 0;
            let mut has_normal_node = false;

            for node in nodes {
                let Some(mut node_guard) = node.lock_ok() else {
                    continue;
                };
                if node_guard.node_type() != NodeType::Normal {
                    continue;
                }

                has_normal_node = true;
                node_guard.set_property(LayeredOptions::LAYERING_LAYER_ID, Some(layer_index));
                node_guard.set_property(
                    LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID,
                    Some(position_index),
                );
                position_index += 1;
            }

            // Mirror Java: layers without normal nodes do not increase layer id.
            if has_normal_node {
                layer_index += 1;
            }
        }

        progress_monitor.done();
    }
}
