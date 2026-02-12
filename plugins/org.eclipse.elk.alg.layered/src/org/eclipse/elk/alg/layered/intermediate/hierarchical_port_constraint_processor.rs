#![allow(clippy::mutable_key_type)]

use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::alignment::Alignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::{Origin, PortRefKey};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions};

const DUMMY_INPUT_PORT: usize = 0;
const DUMMY_OUTPUT_PORT: usize = 1;

pub struct HierarchicalPortConstraintProcessor;

impl ILayoutProcessor<LGraph> for HierarchicalPortConstraintProcessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Hierarchical port constraint processing", 1.0);

        process_eastern_and_western_port_dummies(layered_graph);
        process_northern_and_southern_port_dummies(layered_graph);

        monitor.done();
    }
}

fn process_eastern_and_western_port_dummies(layered_graph: &mut LGraph) {
    let port_constraints = layered_graph
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::Undefined);
    if !port_constraints.is_order_fixed() {
        return;
    }

    let layers = layered_graph.layers().clone();
    if layers.is_empty() {
        return;
    }

    process_eastern_and_western_port_dummies_layer(&layers[0]);
    if layers.len() > 1 {
        process_eastern_and_western_port_dummies_layer(&layers[layers.len() - 1]);
    }
}

fn process_eastern_and_western_port_dummies_layer(layer: &LayerRef) {
    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    let mut nodes_sorted = nodes;
    nodes_sorted.sort_by(|node1, node2| {
        let (node1_type, node1_pos) = node1
            .lock()
            .ok()
            .map(|mut node_guard| {
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                        .unwrap_or(0.0),
                )
            })
            .unwrap_or((NodeType::Normal, 0.0));
        let (node2_type, node2_pos) = node2
            .lock()
            .ok()
            .map(|mut node_guard| {
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::PORT_RATIO_OR_POSITION)
                        .unwrap_or(0.0),
                )
            })
            .unwrap_or((NodeType::Normal, 0.0));

        if node2_type != NodeType::ExternalPort {
            return std::cmp::Ordering::Less;
        }
        if node1_type != NodeType::ExternalPort {
            return std::cmp::Ordering::Greater;
        }

        node1_pos
            .partial_cmp(&node2_pos)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut last_hierarchical_dummy: Option<LNodeRef> = None;

    for node in nodes_sorted {
        let (node_type, ext_side) = node
            .lock()
            .ok()
            .map(|mut node_guard| {
                (
                    node_guard.node_type(),
                    node_guard
                        .get_property(InternalProperties::EXT_PORT_SIDE)
                        .unwrap_or(PortSide::Undefined),
                )
            })
            .unwrap_or((NodeType::Normal, PortSide::Undefined));

        if node_type != NodeType::ExternalPort {
            break;
        }

        if ext_side != PortSide::West && ext_side != PortSide::East {
            continue;
        }

        if let Some(last_dummy) = &last_hierarchical_dummy {
            if let Ok(mut last_guard) = last_dummy.lock() {
                let mut constraints = last_guard
                    .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                    .unwrap_or_default();
                constraints.push(node.clone());
                last_guard.set_property(
                    InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                    Some(constraints),
                );
            }
        }

        last_hierarchical_dummy = Some(node);
    }
}

fn process_northern_and_southern_port_dummies(layered_graph: &mut LGraph) {
    let port_constraints = layered_graph
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::Undefined);
    if !port_constraints.is_side_fixed() {
        return;
    }

    let layers = layered_graph.layers().clone();
    let layer_count = layers.len();
    if layer_count == 0 {
        return;
    }

    let graph_ref = layers
        .first()
        .and_then(|layer| layer.lock().ok().and_then(|layer_guard| layer_guard.graph()));
    let Some(graph_ref) = graph_ref else {
        return;
    };

    let mut ext_port_to_dummy_node_map: Vec<HashMap<PortRefKey, LNodeRef>> =
        vec![HashMap::new(); layer_count + 2];
    let mut new_dummy_nodes: Vec<Vec<LNodeRef>> = vec![Vec::new(); layer_count + 2];
    let mut original_external_port_dummies: Vec<LNodeRef> = Vec::new();

    for curr_layer_idx in 0..layer_count {
        let current_layer = layers[curr_layer_idx].clone();
        let (left_maps, right_maps) = ext_port_to_dummy_node_map.split_at_mut(curr_layer_idx + 2);
        let prev_map = &mut left_maps[curr_layer_idx];
        let next_map = &mut right_maps[0];

        let (left_lists, right_lists) = new_dummy_nodes.split_at_mut(curr_layer_idx + 2);
        let prev_new_nodes = &mut left_lists[curr_layer_idx];
        let next_new_nodes = &mut right_lists[0];

        let layer_nodes = current_layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();

        for current_node in layer_nodes {
            if is_northern_or_southern_dummy(&current_node) {
                original_external_port_dummies.push(current_node);
                continue;
            }

            let incoming_edges = current_node
                .lock()
                .ok()
                .map(|node_guard| node_guard.incoming_edges())
                .unwrap_or_default();

            for edge in incoming_edges {
                let source_node = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.source())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                let Some(source_node) = source_node else {
                    continue;
                };
                if !is_northern_or_southern_dummy(&source_node) {
                    continue;
                }

                let origin_key = origin_port_key(&source_node);
                let Some(origin_key) = origin_key else {
                    continue;
                };

                let prev_layer_dummy = if let Some(existing) = prev_map.get(&origin_key) {
                    existing.clone()
                } else {
                    let dummy = create_dummy(&graph_ref, &source_node);
                    prev_map.insert(origin_key, dummy.clone());
                    prev_new_nodes.push(dummy.clone());
                    dummy
                };

                let dummy_port = prev_layer_dummy
                    .lock()
                    .ok()
                    .and_then(|dummy_guard| dummy_guard.ports().get(DUMMY_OUTPUT_PORT).cloned());
                if let Some(dummy_port) = dummy_port {
                    LEdge::set_source(&edge, Some(dummy_port));
                }
            }

            let outgoing_edges = current_node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();

            for edge in outgoing_edges {
                let target_node = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                let Some(target_node) = target_node else {
                    continue;
                };
                if !is_northern_or_southern_dummy(&target_node) {
                    continue;
                }

                let origin_key = origin_port_key(&target_node);
                let Some(origin_key) = origin_key else {
                    continue;
                };

                let next_layer_dummy = if let Some(existing) = next_map.get(&origin_key) {
                    existing.clone()
                } else {
                    let dummy = create_dummy(&graph_ref, &target_node);
                    next_map.insert(origin_key, dummy.clone());
                    next_new_nodes.push(dummy.clone());
                    dummy
                };

                let dummy_port = next_layer_dummy
                    .lock()
                    .ok()
                    .and_then(|dummy_guard| dummy_guard.ports().get(DUMMY_INPUT_PORT).cloned());
                if let Some(dummy_port) = dummy_port {
                    LEdge::set_target(&edge, Some(dummy_port));
                }
            }
        }
    }

    for (index, node_list) in new_dummy_nodes.iter().enumerate() {
        if node_list.is_empty() {
            continue;
        }

        let layer = if index == 0 {
            let layer = Layer::new(&graph_ref);
            layered_graph.layers_mut().insert(0, layer.clone());
            layer
        } else if index == new_dummy_nodes.len() - 1 {
            let layer = Layer::new(&graph_ref);
            layered_graph.layers_mut().push(layer.clone());
            layer
        } else {
            layers[index - 1].clone()
        };

        for dummy in node_list {
            LNode::set_layer(dummy, Some(layer.clone()));
        }
    }

    for original_dummy in &original_external_port_dummies {
        LNode::set_layer(original_dummy, None);
    }

    layered_graph.set_property(
        InternalProperties::EXT_PORT_REPLACED_DUMMIES,
        Some(original_external_port_dummies),
    );
}

fn is_northern_or_southern_dummy(node: &LNodeRef) -> bool {
    let (node_type, port_side) = node
        .lock()
        .ok()
        .map(|mut node_guard| {
            (
                node_guard.node_type(),
                node_guard
                    .get_property(InternalProperties::EXT_PORT_SIDE)
                    .unwrap_or(PortSide::Undefined),
            )
        })
        .unwrap_or((NodeType::Normal, PortSide::Undefined));

    node_type == NodeType::ExternalPort && (port_side == PortSide::North || port_side == PortSide::South)
}

fn origin_port_key(node: &LNodeRef) -> Option<PortRefKey> {
    let origin = node
        .lock()
        .ok()
        .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN));
    match origin {
        Some(Origin::LPort(port)) => Some(PortRefKey(port)),
        _ => None,
    }
}

fn create_dummy(graph: &LGraphRef, original_dummy: &LNodeRef) -> LNodeRef {
    let new_dummy = LNode::new(graph);

    if let (Ok(mut new_guard), Ok(mut orig_guard)) = (new_dummy.lock(), original_dummy.lock()) {
        let props = orig_guard.shape().graph_element().properties().clone();
        new_guard
            .shape()
            .graph_element()
            .properties_mut()
            .copy_properties(&props);
    }

    if let Ok(mut new_guard) = new_dummy.lock() {
        new_guard.set_property(
            InternalProperties::EXT_PORT_REPLACED_DUMMY,
            Some(original_dummy.clone()),
        );
        new_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
        new_guard.set_property(LayeredOptions::ALIGNMENT, Some(Alignment::Center));
        new_guard.set_node_type(NodeType::ExternalPort);
    }

    let input_port = LPort::new();
    LPort::set_node(&input_port, Some(new_dummy.clone()));
    if let Ok(mut port_guard) = input_port.lock() {
        port_guard.set_side(PortSide::West);
    }

    let output_port = LPort::new();
    LPort::set_node(&output_port, Some(new_dummy.clone()));
    if let Ok(mut port_guard) = output_port.lock() {
        port_guard.set_side(PortSide::East);
    }

    new_dummy
}
