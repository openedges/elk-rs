use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::unsupported_configuration::UnsupportedConfigurationException;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredOptions,
};

pub struct LayerConstraintPostprocessor;

impl ILayoutProcessor<LGraph> for LayerConstraintPostprocessor {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Layer constraint postprocessing", 1.0);

        let graph_ref = graph_ref_for(layered_graph);
        let layers = layered_graph.layers().clone();
        if !layers.is_empty() {
            let first_layer = layers.first().cloned();
            let last_layer = layers.last().cloned();
            if let (Some(first_layer), Some(last_layer)) = (first_layer, last_layer) {
                let first_label_layer = Layer::new(&graph_ref);
                let last_label_layer = Layer::new(&graph_ref);

                move_first_and_last_nodes(
                    layered_graph,
                    &first_layer,
                    &last_layer,
                    &first_label_layer,
                    &last_label_layer,
                );

                if !first_label_layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().is_empty())
                    .unwrap_or(true)
                {
                    layered_graph.layers_mut().insert(0, first_label_layer);
                }
                if !last_label_layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().is_empty())
                    .unwrap_or(true)
                {
                    layered_graph.layers_mut().push(last_label_layer);
                }
            }
        }

        let hidden_nodes = if layered_graph
            .graph_element()
            .properties()
            .has_property(InternalProperties::HIDDEN_NODES)
        {
            layered_graph.get_property(InternalProperties::HIDDEN_NODES)
        } else {
            None
        };
        if hidden_nodes.as_ref().is_some_and(|nodes| !nodes.is_empty()) {
            let first_separate_layer = Layer::new(&graph_ref);
            let last_separate_layer = Layer::new(&graph_ref);
            let first_external_port_layer = Layer::new(&graph_ref);
            let last_external_port_layer = Layer::new(&graph_ref);

            restore_hidden_nodes(
                hidden_nodes.unwrap_or_default(),
                &first_separate_layer,
                &last_separate_layer,
                &first_external_port_layer,
                &last_external_port_layer,
            );

            if !first_separate_layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(true)
            {
                layered_graph.layers_mut().insert(0, first_separate_layer);
            }
            if !last_separate_layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(true)
            {
                layered_graph.layers_mut().push(last_separate_layer);
            }
            // External port layers go outside separate layers
            if !first_external_port_layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(true)
            {
                layered_graph
                    .layers_mut()
                    .insert(0, first_external_port_layer);
            }
            if !last_external_port_layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(true)
            {
                layered_graph
                    .layers_mut()
                    .push(last_external_port_layer);
            }
        }

        monitor.done();
    }
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock()
            .ok()
            .and_then(|layer_guard| layer_guard.graph())
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock().ok().and_then(|node_guard| node_guard.graph()) {
            return graph_ref;
        }
    }
    LGraph::new()
}

fn move_first_and_last_nodes(
    layered_graph: &mut LGraph,
    first_layer: &LayerRef,
    last_layer: &LayerRef,
    first_label_layer: &LayerRef,
    last_label_layer: &LayerRef,
) {
    for layer in layered_graph.layers().clone() {
        let nodes = layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            match layer_constraint_of(&node) {
                LayerConstraint::First => {
                    throw_up_unless_no_incoming_edges(&node);
                    LNode::set_layer(&node, Some(first_layer.clone()));
                    move_labels_to_label_layer(&node, true, first_label_layer);
                }
                LayerConstraint::Last => {
                    throw_up_unless_no_outgoing_edges(&node);
                    LNode::set_layer(&node, Some(last_layer.clone()));
                    move_labels_to_label_layer(&node, false, last_label_layer);
                }
                LayerConstraint::None
                | LayerConstraint::FirstSeparate
                | LayerConstraint::LastSeparate => {}
            }
        }
    }

    layered_graph.layers_mut().retain(|layer| {
        !layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().is_empty())
            .unwrap_or(true)
    });
}

fn move_labels_to_label_layer(node: &LNodeRef, incoming: bool, label_layer: &LayerRef) {
    let edges = if incoming {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.incoming_edges())
            .unwrap_or_default()
    } else {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.outgoing_edges())
            .unwrap_or_default()
    };

    for edge in edges {
        let possible_label_dummy = if incoming {
            edge.lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
        } else {
            edge.lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
        };
        let Some(possible_label_dummy) = possible_label_dummy else {
            continue;
        };
        let is_label = possible_label_dummy
            .lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::Label)
            .unwrap_or(false);
        if is_label {
            LNode::set_layer(&possible_label_dummy, Some(label_layer.clone()));
        }
    }
}

fn restore_hidden_nodes(
    hidden_nodes: Vec<LNodeRef>,
    first_separate_layer: &LayerRef,
    last_separate_layer: &LayerRef,
    first_external_port_layer: &LayerRef,
    last_external_port_layer: &LayerRef,
) {
    for hidden_node in hidden_nodes {
        let is_external_port = hidden_node
            .lock()
            .ok()
            .map(|ng| ng.node_type() == NodeType::ExternalPort)
            .unwrap_or(false);
        match layer_constraint_of(&hidden_node) {
            LayerConstraint::FirstSeparate => {
                if is_external_port {
                    LNode::set_layer(&hidden_node, Some(first_external_port_layer.clone()));
                } else {
                    LNode::set_layer(&hidden_node, Some(first_separate_layer.clone()));
                }
            }
            LayerConstraint::LastSeparate => {
                if is_external_port {
                    LNode::set_layer(&hidden_node, Some(last_external_port_layer.clone()));
                } else {
                    LNode::set_layer(&hidden_node, Some(last_separate_layer.clone()));
                }
            }
            LayerConstraint::None | LayerConstraint::First | LayerConstraint::Last => {}
        }

        let connected_edges = hidden_node
            .lock()
            .ok()
            .map(|node_guard| node_guard.connected_edges())
            .unwrap_or_default();
        for hidden_edge in connected_edges {
            let source_set = hidden_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source())
                .is_some();
            let target_set = hidden_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .is_some();
            if source_set && target_set {
                continue;
            }

            let is_outgoing = hidden_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .is_none();
            let opposite = hidden_edge.lock().ok().and_then(|mut edge_guard| {
                edge_guard.get_property(InternalProperties::ORIGINAL_OPPOSITE_PORT)
            });
            let Some(opposite) = opposite else {
                continue;
            };
            if is_outgoing {
                LEdge::set_target(&hidden_edge, Some(opposite));
            } else {
                LEdge::set_source(&hidden_edge, Some(opposite));
            }
        }
    }
}

fn throw_up_unless_no_incoming_edges(node: &LNodeRef) {
    let incoming = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.incoming_edges())
        .unwrap_or_default();
    for incoming_edge in incoming {
        let source_type = incoming_edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
            .and_then(|source| source.lock().ok().map(|node_guard| node_guard.node_type()))
            .unwrap_or(NodeType::Normal);
        if source_type != NodeType::Label {
            let designation = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.designation())
                .unwrap_or_else(|| "<unknown>".to_owned());
            panic!(
                "{}",
                UnsupportedConfigurationException::new(format!(
                    "Node '{}' has its layer constraint set to FIRST, but has at least one incoming edge that does not come from a FIRST_SEPARATE node. That must not happen.",
                    designation
                ))
            );
        }
    }
}

fn throw_up_unless_no_outgoing_edges(node: &LNodeRef) {
    let outgoing = node
        .lock()
        .ok()
        .map(|node_guard| node_guard.outgoing_edges())
        .unwrap_or_default();
    for outgoing_edge in outgoing {
        let target_type = outgoing_edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target())
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
            .and_then(|target| target.lock().ok().map(|node_guard| node_guard.node_type()))
            .unwrap_or(NodeType::Normal);
        if target_type != NodeType::Label {
            let designation = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.designation())
                .unwrap_or_else(|| "<unknown>".to_owned());
            panic!(
                "{}",
                UnsupportedConfigurationException::new(format!(
                    "Node '{}' has its layer constraint set to LAST, but has at least one outgoing edge that does not go to a LAST_SEPARATE node. That must not happen.",
                    designation
                ))
            );
        }
    }
}

fn layer_constraint_of(node: &LNodeRef) -> LayerConstraint {
    node.lock()
        .ok()
        .and_then(|mut node_guard| {
            if node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            {
                node_guard.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            } else {
                None
            }
        })
        .unwrap_or(LayerConstraint::None)
}
