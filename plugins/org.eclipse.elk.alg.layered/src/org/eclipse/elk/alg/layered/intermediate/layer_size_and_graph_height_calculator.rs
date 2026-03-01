use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::elk_margin::ElkMargin;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, NodeType};
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;

static TRACE_LAYER_HEIGHT: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_LAYER_HEIGHT").is_some());

pub struct LayerSizeAndGraphHeightCalculator;

impl ILayoutProcessor<LGraph> for LayerSizeAndGraphHeightCalculator {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Layer size calculation", 1.0);
        let trace_layer_height = *TRACE_LAYER_HEIGHT;

        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut found_nodes = false;

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.size().x = 0.0;
                layer_guard.size().y = 0.0;
            }

            if nodes.is_empty() {
                continue;
            }

            found_nodes = true;

            let mut layer_width: f64 = 0.0;
            for node in &nodes {
                if let Ok(mut node_guard) = node.lock() {
                    let size_x = node_guard.shape().size_ref().x;
                    let margin = node_guard.margin().clone();
                    layer_width = layer_width.max(size_x + margin.left + margin.right);
                }
            }

            let mut top = 0.0;
            if let Ok(mut first_node_guard) = nodes[0].lock() {
                let pos_y = first_node_guard.shape().position_ref().y;
                let margin_top = first_node_guard.margin().top;
                top = pos_y - margin_top;
                if first_node_guard.node_type() == NodeType::ExternalPort {
                    let spacing = spacing_ports_surrounding(layered_graph);
                    top -= spacing.top;
                }
            }

            let mut bottom = 0.0;
            if let Some(last_node) = nodes.last() {
                if let Ok(mut last_node_guard) = last_node.lock() {
                    let pos_y = last_node_guard.shape().position_ref().y;
                    let size_y = last_node_guard.shape().size_ref().y;
                    let margin_bottom = last_node_guard.margin().bottom;
                    bottom = pos_y + size_y + margin_bottom;
                    if last_node_guard.node_type() == NodeType::ExternalPort {
                        let spacing = spacing_ports_surrounding(layered_graph);
                        bottom += spacing.bottom;
                    }
                }
            }

            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.size().x = layer_width;
                layer_guard.size().y = bottom - top;
            }

            if trace_layer_height {
                let first = nodes.first().and_then(|node| {
                    node.lock().ok().map(|mut g| {
                        (
                            g.shape().graph_element().id,
                            g.node_type(),
                            g.shape().position_ref().y,
                            g.shape().size_ref().y,
                            g.margin().top,
                            g.margin().bottom,
                        )
                    })
                });
                let last = nodes.last().and_then(|node| {
                    node.lock().ok().map(|mut g| {
                        (
                            g.shape().graph_element().id,
                            g.node_type(),
                            g.shape().position_ref().y,
                            g.shape().size_ref().y,
                            g.margin().top,
                            g.margin().bottom,
                        )
                    })
                });
                eprintln!(
                    "[layer-height] layer nodes={} top={:.1} bottom={:.1} first={:?} last={:?}",
                    nodes.len(),
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
