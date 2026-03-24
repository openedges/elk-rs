use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_margin::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{ArenaSync, LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

pub struct LayerSizeAndGraphHeightCalculator;

impl ILayoutProcessor<LGraph> for LayerSizeAndGraphHeightCalculator {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Layer size calculation", 1.0);
        let trace_layer_height = ElkTrace::global().layer_height;

        let spacing = spacing_ports_surrounding(layered_graph);
        let mut sync = ArenaSync::from_lgraph(layered_graph);

        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut found_nodes = false;

        let layer_ids: Vec<_> = sync.arena().all_layer_ids().collect();
        for layer_id in layer_ids {
            sync.arena_mut().layer_size_mut(layer_id).x = 0.0;
            sync.arena_mut().layer_size_mut(layer_id).y = 0.0;

            let node_ids: Vec<_> = sync.arena().layer_nodes(layer_id).to_vec();
            if node_ids.is_empty() {
                continue;
            }

            found_nodes = true;

            let mut layer_width: f64 = 0.0;
            for &nid in &node_ids {
                let size_x = sync.arena().node_size(nid).x;
                let margin = sync.arena().node_margin(nid);
                layer_width = layer_width.max(size_x + margin.left + margin.right);
            }

            let first_nid = node_ids[0];
            let top = {
                let pos_y = sync.arena().node_pos(first_nid).y;
                let margin_top = sync.arena().node_margin(first_nid).top;
                let mut t = pos_y - margin_top;
                if sync.arena().node_type(first_nid) == NodeType::ExternalPort {
                    t -= spacing.top;
                }
                t
            };

            let last_nid = *node_ids.last().unwrap();
            let bottom = {
                let pos_y = sync.arena().node_pos(last_nid).y;
                let size_y = sync.arena().node_size(last_nid).y;
                let margin_bottom = sync.arena().node_margin(last_nid).bottom;
                let mut b = pos_y + size_y + margin_bottom;
                if sync.arena().node_type(last_nid) == NodeType::ExternalPort {
                    b += spacing.bottom;
                }
                b
            };

            sync.arena_mut().layer_size_mut(layer_id).x = layer_width;
            sync.arena_mut().layer_size_mut(layer_id).y = bottom - top;

            if trace_layer_height {
                let first = {
                    let nid = node_ids[0];
                    (
                        sync.arena().node_element_id(nid),
                        sync.arena().node_type(nid),
                        sync.arena().node_pos(nid).y,
                        sync.arena().node_size(nid).y,
                        sync.arena().node_margin(nid).top,
                        sync.arena().node_margin(nid).bottom,
                    )
                };
                let last = {
                    let nid = *node_ids.last().unwrap();
                    (
                        sync.arena().node_element_id(nid),
                        sync.arena().node_type(nid),
                        sync.arena().node_pos(nid).y,
                        sync.arena().node_size(nid).y,
                        sync.arena().node_margin(nid).top,
                        sync.arena().node_margin(nid).bottom,
                    )
                };
                eprintln!(
                    "[layer-height] layer nodes={} top={:.1} bottom={:.1} first={:?} last={:?}",
                    node_ids.len(),
                    top,
                    bottom,
                    first,
                    last
                );
            }

            min_y = min_y.min(top);
            max_y = max_y.max(bottom);
        }

        if !found_nodes {
            min_y = 0.0;
            max_y = 0.0;
        }

        // Sync layer sizes back to Arc graph
        sync.sync_layer_sizes_to_graph();

        // Graph-level writes remain on &mut LGraph
        layered_graph.size().y = max_y - min_y;
        layered_graph.offset().y -= min_y;
        if trace_layer_height {
            eprintln!(
                "[layer-height] result min_y={:.1} max_y={:.1} size_y={:.1} offset_y={:.1}",
                min_y,
                max_y,
                layered_graph.size_ref().y,
                layered_graph.offset_ref().y
            );
        }

        monitor.done();
    }
}

fn spacing_ports_surrounding(layered_graph: &mut LGraph) -> ElkMargin {
    if layered_graph
        .graph_element()
        .properties()
        .has_property(LayeredOptions::SPACING_PORTS_SURROUNDING)
    {
        layered_graph
            .get_property(LayeredOptions::SPACING_PORTS_SURROUNDING)
            .unwrap_or_default()
    } else {
        ElkMargin::new()
    }
}
