use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LayerRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

pub struct HierarchicalPortPositionProcessor;

impl ILayoutProcessor<LGraph> for HierarchicalPortPositionProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Hierarchical port position processing", 1.0);

        let layers = layered_graph.layers().clone();
        if !layers.is_empty() {
            fix_coordinates(&layers[0], layered_graph);
        }
        if layers.len() > 1 {
            fix_coordinates(&layers[layers.len() - 1], layered_graph);
        }

        monitor.done();
    }
}

fn fix_coordinates(layer: &LayerRef, layered_graph: &mut LGraph) {
    let port_constraints = layered_graph
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::Undefined);
    if !(port_constraints.is_ratio_fixed() || port_constraints.is_pos_fixed()) {
        return;
    }

    let graph_height = layered_graph.actual_size().y;

    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    for node in nodes {
        let (node_type, ext_side, ratio_or_pos, anchor) = node
            .lock()
            .ok()
            .map(|mut node_guard| {
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined),
                    node_guard
                        .get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                        .unwrap_or(0.0),
                    node_guard
                        .get_property(LayeredOptions::PORT_ANCHOR)
                        .unwrap_or_default(),
                )
            })
            .unwrap_or((NodeType::Normal, PortSide::Undefined, 0.0, Default::default()));

        if node_type != NodeType::ExternalPort {
            continue;
        }
        if ext_side != PortSide::East && ext_side != PortSide::West {
            continue;
        }

        let mut final_y = ratio_or_pos;
        if port_constraints == PortConstraints::FixedRatio {
            final_y *= graph_height;
        }

        if let Ok(mut node_guard) = node.lock() {
            node_guard.shape().position().y = final_y - anchor.y;
            node_guard.border_to_content_area_coordinates(false, true);
        }
    }
}
