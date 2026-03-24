use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{ArenaSync, LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct ConstraintsPostprocessor;

impl ILayoutProcessor<LGraph> for ConstraintsPostprocessor {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Constraints Postprocessor", 1.0);

        let mut sync = ArenaSync::from_lgraph(graph);

        let mut layer_index: i32 = 0;
        let layer_ids: Vec<_> = sync.arena().all_layer_ids().collect();
        for layer_id in layer_ids {
            let node_ids: Vec<_> = sync.arena().layer_nodes(layer_id).to_vec();

            let mut position_index: i32 = 0;
            let mut has_normal_node = false;

            for &nid in &node_ids {
                if sync.arena().node_type(nid) != NodeType::Normal {
                    continue;
                }

                has_normal_node = true;
                sync.arena_mut()
                    .node_properties_mut(nid)
                    .set_property(LayeredOptions::LAYERING_LAYER_ID, Some(layer_index));
                sync.arena_mut().node_properties_mut(nid).set_property(
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

        // Sync node properties back to Arc graph
        sync.sync_node_properties_to_graph();

        progress_monitor.done();
    }
}
