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

                if !{
                    let layer_guard = first_label_layer.lock();
                    layer_guard.nodes().is_empty()
                } {
                    layered_graph.layers_mut().insert(0, first_label_layer);
                }
                if !{
                    let layer_guard = last_label_layer.lock();
                    layer_guard.nodes().is_empty()
                } {
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

            restore_hidden_nodes(
                hidden_nodes.unwrap_or_default(),
                &first_separate_layer,
                &last_separate_layer,
            );

            if !{
                let layer_guard = first_separate_layer.lock();
                layer_guard.nodes().is_empty()
            } {
                layered_graph.layers_mut().insert(0, first_separate_layer);
            }
            if !{
                let layer_guard = last_separate_layer.lock();
                layer_guard.nodes().is_empty()
            } {
                layered_graph.layers_mut().push(last_separate_layer);
            }
        }

        monitor.done();
    }
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock().graph()
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock().graph() {
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
            .lock().nodes().clone();
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
        let layer_guard = layer.lock();
        !layer_guard.nodes().is_empty()
    });
}

fn move_labels_to_label_layer(node: &LNodeRef, incoming: bool, label_layer: &LayerRef) {
    let edges = if incoming {
        node.lock().incoming_edges()
    } else {
        node.lock().outgoing_edges()
    };

    for edge in edges {
        let possible_label_dummy = if incoming {
            edge.lock().source()
                .and_then(|port| port.lock().node())
        } else {
            edge.lock().target()
                .and_then(|port| port.lock().node())
        };
        let Some(possible_label_dummy) = possible_label_dummy else {
            continue;
        };
        let is_label = {
            let node_guard = possible_label_dummy.lock();
            node_guard.node_type() == NodeType::Label
        };
        if is_label {
            LNode::set_layer(&possible_label_dummy, Some(label_layer.clone()));
        }
    }
}

fn restore_hidden_nodes(
    hidden_nodes: Vec<LNodeRef>,
    first_separate_layer: &LayerRef,
    last_separate_layer: &LayerRef,
) {
    for hidden_node in hidden_nodes {
        match layer_constraint_of(&hidden_node) {
            LayerConstraint::FirstSeparate => {
                LNode::set_layer(&hidden_node, Some(first_separate_layer.clone()));
            }
            LayerConstraint::LastSeparate => {
                LNode::set_layer(&hidden_node, Some(last_separate_layer.clone()));
            }
            LayerConstraint::None | LayerConstraint::First | LayerConstraint::Last => {}
        }

        let connected_edges = hidden_node
            .lock().connected_edges();
        for hidden_edge in connected_edges {
            let source_set = hidden_edge
                .lock().source()
                .is_some();
            let target_set = hidden_edge
                .lock().target()
                .is_some();
            if source_set && target_set {
                continue;
            }

            let is_outgoing = hidden_edge
                .lock().target()
                .is_none();
            let opposite = {
                let mut edge_guard = hidden_edge.lock();
                edge_guard.get_property(InternalProperties::ORIGINAL_OPPOSITE_PORT)
            };
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
        .lock().incoming_edges();
    for incoming_edge in incoming {
        let source_type = incoming_edge
            .lock().source()
            .and_then(|port| port.lock().node())
            .map(|source| source.lock().node_type())
            .unwrap_or(NodeType::Normal);
        if source_type != NodeType::Label {
            let designation = {
                let node_guard = node.lock();
                node_guard.designation()
            };
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
        .lock().outgoing_edges();
    for outgoing_edge in outgoing {
        let target_type = outgoing_edge
            .lock().target()
            .and_then(|port| port.lock().node())
            .map(|target| target.lock().node_type())
            .unwrap_or(NodeType::Normal);
        if target_type != NodeType::Label {
            let designation = {
                let node_guard = node.lock();
                node_guard.designation()
            };
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
    let mut node_guard = node.lock();
    if node_guard
        .shape()
        .graph_element()
        .properties()
        .has_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
    {
        node_guard.get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
            .unwrap_or(LayerConstraint::None)
    } else {
        LayerConstraint::None
    }
}
