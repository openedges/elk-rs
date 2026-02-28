use std::sync::Arc;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_label_placement::EdgeLabelPlacement;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LNode, LPort, LPortRef, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, Origin, PortType,
};

pub struct InvertedPortProcessor;

impl ILayoutProcessor<LGraph> for InvertedPortProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Inverted port preprocessing", 1.0);

        let layers = layered_graph.layers().clone();
        for layer in layers {
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();

            for node in nodes {
                let skip = node
                    .lock()
                    .ok()
                    .map(|mut node_guard| {
                        if node_guard.node_type() != NodeType::Normal {
                            return true;
                        }
                        if !node_guard
                            .shape()
                            .graph_element()
                            .properties()
                            .has_property(LayeredOptions::PORT_CONSTRAINTS)
                        {
                            return true;
                        }
                        !node_guard
                            .get_property(LayeredOptions::PORT_CONSTRAINTS)
                            .unwrap_or(PortConstraints::Undefined)
                            .is_side_fixed()
                    })
                    .unwrap_or(true);
                if skip {
                    continue;
                }

                let east_input_ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| {
                        node_guard.ports_by_type_and_side(PortType::Input, PortSide::East)
                    })
                    .unwrap_or_default();
                for east_port in east_input_ports {
                    let incoming = east_port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.incoming_edges().clone())
                        .unwrap_or_default();
                    for edge in incoming {
                        create_east_port_side_dummy(&layer, &east_port, &edge);
                    }
                }

                let west_output_ports = node
                    .lock()
                    .ok()
                    .map(|node_guard| {
                        node_guard.ports_by_type_and_side(PortType::Output, PortSide::West)
                    })
                    .unwrap_or_default();
                for west_port in west_output_ports {
                    let outgoing = west_port
                        .lock()
                        .ok()
                        .map(|port_guard| port_guard.outgoing_edges().clone())
                        .unwrap_or_default();
                    for edge in outgoing {
                        create_west_port_side_dummy(&layer, &west_port, &edge);
                    }
                }
            }
        }

        monitor.done();
    }
}

fn create_east_port_side_dummy(layer: &LayerRef, east_port: &LPortRef, edge: &LEdgeRef) {
    let source_node = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.source())
        .and_then(|source| source.lock().ok().and_then(|port_guard| port_guard.node()));
    let target_node = east_port
        .lock()
        .ok()
        .and_then(|port_guard| port_guard.node());
    if source_node
        .as_ref()
        .zip(target_node.as_ref())
        .map(|(source, target)| std::sync::Arc::ptr_eq(source, target))
        .unwrap_or(false)
    {
        return;
    }

    let dummy = create_dummy_node(layer, edge);
    let dummy_input = create_dummy_port(&dummy, PortSide::West);
    let dummy_output = create_dummy_port(&dummy, PortSide::East);

    LEdge::set_target(edge, Some(dummy_input));

    let dummy_edge = LEdge::new();
    if let (Ok(mut dummy_edge_guard), Ok(mut old_edge_guard)) = (dummy_edge.lock(), edge.lock()) {
        dummy_edge_guard
            .graph_element()
            .properties_mut()
            .copy_properties(old_edge_guard.graph_element().properties());
        dummy_edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, None::<KVectorChain>);
    }
    LEdge::set_source(&dummy_edge, Some(dummy_output));
    LEdge::set_target(&dummy_edge, Some(east_port.clone()));

    move_head_labels(edge, &dummy_edge);
}

fn create_west_port_side_dummy(layer: &LayerRef, west_port: &LPortRef, edge: &LEdgeRef) {
    let source_node = west_port
        .lock()
        .ok()
        .and_then(|port_guard| port_guard.node());
    let target_node = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.target())
        .and_then(|target| target.lock().ok().and_then(|port_guard| port_guard.node()));
    if source_node
        .as_ref()
        .zip(target_node.as_ref())
        .map(|(source, target)| std::sync::Arc::ptr_eq(source, target))
        .unwrap_or(false)
    {
        return;
    }

    let dummy = create_dummy_node(layer, edge);
    let dummy_input = create_dummy_port(&dummy, PortSide::West);
    let dummy_output = create_dummy_port(&dummy, PortSide::East);

    let original_target = edge.lock().ok().and_then(|edge_guard| edge_guard.target());
    LEdge::set_target(edge, Some(dummy_input));

    let dummy_edge = LEdge::new();
    if let (Ok(mut dummy_edge_guard), Ok(mut old_edge_guard)) = (dummy_edge.lock(), edge.lock()) {
        dummy_edge_guard
            .graph_element()
            .properties_mut()
            .copy_properties(old_edge_guard.graph_element().properties());
        dummy_edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, None::<KVectorChain>);
    }
    LEdge::set_source(&dummy_edge, Some(dummy_output));
    LEdge::set_target(&dummy_edge, original_target);

    move_head_labels(edge, &dummy_edge);
}

fn create_dummy_node(
    layer: &LayerRef,
    origin_edge: &LEdgeRef,
) -> Arc<Mutex<LNode>> {
    let graph = layer
        .lock()
        .ok()
        .and_then(|layer_guard| layer_guard.graph())
        .unwrap_or_default();
    let dummy = LNode::new(&graph);
    if let Ok(mut dummy_guard) = dummy.lock() {
        dummy_guard.set_node_type(NodeType::LongEdge);
        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LEdge(origin_edge.clone())),
        );
        dummy_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
    }
    LNode::set_layer(&dummy, Some(layer.clone()));
    dummy
}

fn create_dummy_port(dummy: &Arc<Mutex<LNode>>, side: PortSide) -> LPortRef {
    let port = LPort::new();
    if let Ok(mut port_guard) = port.lock() {
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(dummy.clone()));
    port
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
                .retain(|candidate| !std::sync::Arc::ptr_eq(candidate, &label));
        }
        if let Ok(mut new_edge_guard) = new_edge.lock() {
            new_edge_guard.labels_mut().push(label);
        }
    }
}
