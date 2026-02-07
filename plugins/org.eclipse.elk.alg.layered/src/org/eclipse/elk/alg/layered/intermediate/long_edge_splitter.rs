use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LNode, LNodeRef, LPort, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};

pub struct LongEdgeSplitter;

impl ILayoutProcessor<LGraph> for LongEdgeSplitter {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Edge splitting", 1.0);

        let layers = layered_graph.layers().clone();
        if layers.len() <= 2 {
            monitor.done();
            return;
        }

        for layer_index in 0..(layers.len() - 1) {
            let layer = layers[layer_index].clone();
            let next_layer = layers[layer_index + 1].clone();
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.ports().clone())
                    .unwrap_or_default();
                for port in ports {
                    let outgoing = port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.outgoing_edges().clone())
                        .unwrap_or_default();
                    for edge in outgoing {
                        let target_layer_index = target_layer_index(&edge, &layers);
                        if target_layer_index > layer_index + 1 {
                            let dummy = create_dummy_node(&next_layer, &edge);
                            Self::split_edge(&edge, &dummy);
                        }
                    }
                }
            }
        }

        monitor.done();
    }
}

impl LongEdgeSplitter {
    pub fn split_edge(edge: &LEdgeRef, dummy_node: &LNodeRef) -> LEdgeRef {
        let old_edge_target = edge.lock().ok().and_then(|edge_guard| edge_guard.target());
        let mut thickness = edge
            .lock()
            .ok()
            .and_then(|mut edge_guard| {
                if edge_guard
                    .graph_element()
                    .properties()
                    .has_property(CoreOptions::EDGE_THICKNESS)
                {
                    edge_guard.get_property(CoreOptions::EDGE_THICKNESS)
                } else {
                    None
                }
            })
            .unwrap_or(1.0);
        if thickness < 0.0 {
            thickness = 0.0;
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(thickness));
            }
        }

        if let Ok(mut dummy_guard) = dummy_node.lock() {
            dummy_guard.shape().size().y = thickness;
        }
        let port_pos = (thickness / 2.0).floor();

        let dummy_input = LPort::new();
        if let Ok(mut input_guard) = dummy_input.lock() {
            input_guard.set_side(PortSide::West);
            input_guard.shape().position().y = port_pos;
        }
        LPort::set_node(&dummy_input, Some(dummy_node.clone()));

        let dummy_output = LPort::new();
        if let Ok(mut output_guard) = dummy_output.lock() {
            output_guard.set_side(PortSide::East);
            output_guard.shape().position().y = port_pos;
        }
        LPort::set_node(&dummy_output, Some(dummy_node.clone()));

        LEdge::set_target(edge, Some(dummy_input));

        let dummy_edge = LEdge::new();
        if let (Ok(mut new_edge), Ok(mut old_edge)) = (dummy_edge.lock(), edge.lock()) {
            new_edge
                .graph_element()
                .properties_mut()
                .copy_properties(old_edge.graph_element().properties());
            new_edge.set_property(LayeredOptions::JUNCTION_POINTS, None::<KVectorChain>);
        }

        LEdge::set_source(&dummy_edge, Some(dummy_output));
        LEdge::set_target(&dummy_edge, old_edge_target);

        move_head_labels(edge, &dummy_edge);
        dummy_edge
    }
}

fn create_dummy_node(target_layer: &LayerRef, edge_to_split: &LEdgeRef) -> LNodeRef {
    let graph = target_layer
        .lock()
        .ok()
        .and_then(|layer_guard| layer_guard.graph())
        .unwrap_or_default();
    let dummy = LNode::new(&graph);
    if let Ok(mut dummy_guard) = dummy.lock() {
        dummy_guard.set_node_type(NodeType::LongEdge);
        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LEdge(edge_to_split.clone())),
        );
        dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
    }
    LNode::set_layer(&dummy, Some(target_layer.clone()));
    dummy
}

fn target_layer_index(edge: &LEdgeRef, layers: &[LayerRef]) -> usize {
    let target_layer = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.target())
        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
        .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()));
    let Some(target_layer) = target_layer else {
        return 0;
    };
    layers
        .iter()
        .position(|layer| Arc::ptr_eq(layer, &target_layer))
        .unwrap_or(0)
}

fn move_head_labels(old_edge: &LEdgeRef, new_edge: &LEdgeRef) {
    let labels = old_edge
        .lock()
        .ok()
        .map(|edge_guard| edge_guard.labels().clone())
        .unwrap_or_default();
    for label in labels {
        let placement = label
            .lock()
            .ok()
            .and_then(|mut label_guard| {
                if label_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                {
                    label_guard.get_property(LayeredOptions::EDGE_LABELS_PLACEMENT)
                } else {
                    None
                }
            })
            .unwrap_or(EdgeLabelPlacement::Center);
        if placement != EdgeLabelPlacement::Head {
            continue;
        }

        if let Ok(mut old_edge_guard) = old_edge.lock() {
            old_edge_guard
                .labels_mut()
                .retain(|candidate| !Arc::ptr_eq(candidate, &label));
        }
        if let Ok(mut new_edge_guard) = new_edge.lock() {
            new_edge_guard.labels_mut().push(label);
        }
    }
}
