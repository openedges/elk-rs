use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::LongEdgeSplitter;
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, LayeredOptions, Origin};

pub struct AlternatingLayerUnzipper;

impl ILayoutProcessor<LGraph> for AlternatingLayerUnzipper {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Alternating layer unzipping", 1.0);

        let graph_ref = graph_ref_for(graph);
        let mut insertion_layer_offset: usize = 1;
        let mut new_layers: Vec<(LayerRef, usize)> = Vec::new();
        let original_layer_count = graph.layers().len();

        for i in 0..original_layer_count {
            let Some(layer) = graph.layers().get(i).cloned() else {
                continue;
            };

            let n = get_layer_split_property(&layer);
            let reset_on_long_edges = get_reset_on_long_edges_property(&layer);
            let minimize_edge_length = get_minimize_edge_length_property(&layer);

            if n <= 0 {
                continue;
            }

            if minimize_edge_length {
                let layer_nodes = layer
                    .lock_ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default();
                if !layer_nodes.is_empty() {
                    let mut max_width: f64 = 0.0;
                    let mut average_height: f64 = 0.0;

                    for node in &layer_nodes {
                        if let Some(mut node_guard) = node.lock_ok() {
                            max_width = max_width.max(node_guard.shape().size_ref().x);
                            average_height += node_guard.shape().size_ref().y;
                        }
                    }
                    average_height /= layer_nodes.len() as f64;

                    let spacing_edge_node_between_layers = graph
                        .get_property(LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS)
                        .unwrap_or(0.0);
                    let spacing_edge_edge_between_layers = graph
                        .get_property(LayeredOptions::SPACING_EDGE_EDGE_BETWEEN_LAYERS)
                        .unwrap_or(0.0);
                    let spacing_node_node_between_layers = graph
                        .get_property(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS)
                        .unwrap_or(0.0);
                    let spacing_node_node = graph
                        .get_property(LayeredOptions::SPACING_NODE_NODE)
                        .unwrap_or(0.0);
                    let spacing_edge_node = graph
                        .get_property(LayeredOptions::SPACING_EDGE_NODE)
                        .unwrap_or(0.0);

                    max_width += (2.0 * spacing_edge_node_between_layers).max(
                        ((layer_nodes.len() as f64) * spacing_edge_edge_between_layers)
                            .max(spacing_node_node_between_layers),
                    );
                    average_height += spacing_node_node.max(spacing_edge_node);

                    if max_width / average_height >= (layer_nodes.len() as f64) / 4.0 {
                        continue;
                    }
                }
            }

            let layer_node_count = layer
                .lock_ok()
                .map(|layer_guard| layer_guard.nodes().len())
                .unwrap_or(0);

            if layer_node_count > n as usize {
                let mut sub_layers = Vec::new();
                sub_layers.push(layer.clone());

                for j in 0..((n as usize) - 1) {
                    let new_layer = Layer::new(&graph_ref);
                    new_layers.push((new_layer.clone(), i + j + insertion_layer_offset));
                    sub_layers.push(new_layer);
                }
                insertion_layer_offset += (n as usize) - 1;

                let nodes_in_layer = sub_layers[0]
                    .lock_ok()
                    .map(|layer_guard| layer_guard.nodes().len())
                    .unwrap_or(0);

                let mut j: isize = 0;
                let mut node_index: isize = 0;
                let mut target_layer: isize = 0;

                while j < nodes_in_layer as isize {
                    let Some(node) = sub_layers[0].lock_ok().and_then(|layer_guard| {
                        layer_guard.nodes().get(node_index as usize).cloned()
                    }) else {
                        break;
                    };

                    let node_type = node
                        .lock_ok()
                        .map(|node_guard| node_guard.node_type())
                        .unwrap_or(NodeType::Normal);

                    if node_type != NodeType::NonshiftingPlaceholder {
                        let shifted = shift_node(
                            &graph_ref,
                            &sub_layers,
                            target_layer.rem_euclid(n as isize) as usize,
                            node_index as usize,
                        );
                        node_index += shifted as isize;
                    } else {
                        j -= 1;
                        target_layer -= 1;
                    }

                    if reset_on_long_edges && node_type == NodeType::LongEdge {
                        target_layer = -1;
                    }

                    j += 1;
                    node_index += 1;
                    target_layer += 1;
                }
            }
        }

        for (new_layer, insertion_index) in new_layers {
            let index = insertion_index.min(graph.layers().len());
            graph.layers_mut().insert(index, new_layer);
        }

        remove_unconnected_placeholder_nodes(graph);
        progress_monitor.done();
    }
}

fn get_layer_split_property(layer: &LayerRef) -> i32 {
    let mut layer_split = i32::MAX;
    let mut property_unset = true;

    let nodes = layer
        .lock_ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    for node in nodes {
        if has_node_property(&node, LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT) {
            property_unset = false;
            let node_value = node
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT)
                })
                .unwrap_or(1);
            layer_split = layer_split.min(node_value);
        }
    }

    if property_unset {
        return LayeredOptions::LAYER_UNZIPPING_LAYER_SPLIT
            .get_default()
            .unwrap_or(2);
    }

    layer_split
}

fn get_reset_on_long_edges_property(layer: &LayerRef) -> bool {
    let nodes = layer
        .lock_ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    for node in nodes {
        if has_node_property(&node, LayeredOptions::LAYER_UNZIPPING_RESET_ON_LONG_EDGES) {
            let reset_on_long_edges = node
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::LAYER_UNZIPPING_RESET_ON_LONG_EDGES)
                })
                .unwrap_or(true);
            if !reset_on_long_edges {
                return false;
            }
        }
    }

    true
}

fn get_minimize_edge_length_property(layer: &LayerRef) -> bool {
    let nodes = layer
        .lock_ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    for node in nodes {
        if has_node_property(&node, LayeredOptions::LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH) {
            let minimize_edge_length = node
                .lock_ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(LayeredOptions::LAYER_UNZIPPING_MINIMIZE_EDGE_LENGTH)
                })
                .unwrap_or(false);
            if minimize_edge_length {
                return true;
            }
        }
    }

    false
}

fn shift_node(
    graph_ref: &LGraphRef,
    sub_layers: &[LayerRef],
    target_layer: usize,
    node_index: usize,
) -> usize {
    let Some(node) = sub_layers[0]
        .lock_ok()
        .and_then(|layer_guard| layer_guard.nodes().get(node_index).cloned())
    else {
        return 0;
    };

    if target_layer > 0 {
        LNode::set_layer(&node, Some(sub_layers[target_layer].clone()));
    }

    let mut edge_count = 0;
    let mut no_incoming_edges = true;

    let reversed_incoming_edges: Vec<LEdgeRef> = node
        .lock_ok()
        .map(|node_guard| node_guard.incoming_edges())
        .unwrap_or_default()
        .into_iter()
        .rev()
        .collect();
    for incoming_edge in reversed_incoming_edges {
        no_incoming_edges = false;
        let mut next_edge_to_split = incoming_edge;
        for (layer_index, layer) in sub_layers.iter().enumerate().take(target_layer) {
            let dummy_node = create_dummy_node(graph_ref, &next_edge_to_split);
            place_node_at_index(&dummy_node, layer, node_index + edge_count);
            next_edge_to_split = LongEdgeSplitter::split_edge(&next_edge_to_split, &dummy_node);
            if layer_index + 1 == target_layer {
                break;
            }
        }
        if target_layer > 0 {
            edge_count += 1;
        }
    }

    if no_incoming_edges {
        for layer in sub_layers.iter().take(target_layer) {
            let dummy_node = LNode::new(graph_ref);
            if let Some(mut dummy_guard) = dummy_node.lock_ok() {
                dummy_guard.set_node_type(NodeType::Placeholder);
            }
            place_node_at_index(&dummy_node, layer, node_index + edge_count);
        }
        if target_layer > 0 {
            edge_count += 1;
        }
    }

    let mut extra_edge = false;
    let outgoing_edges = node
        .lock_ok()
        .map(|node_guard| node_guard.outgoing_edges())
        .unwrap_or_default();
    for outgoing_edge in outgoing_edges {
        let mut next_edge_to_split = outgoing_edge;
        for layer in sub_layers.iter().skip(target_layer + 1) {
            let dummy_node = create_dummy_node(graph_ref, &next_edge_to_split);
            LNode::set_layer(&dummy_node, Some(layer.clone()));
            next_edge_to_split = LongEdgeSplitter::split_edge(&next_edge_to_split, &dummy_node);
        }

        if extra_edge {
            for layer in sub_layers.iter().take(target_layer + 1) {
                let placeholder = LNode::new(graph_ref);
                if let Some(mut placeholder_guard) = placeholder.lock_ok() {
                    placeholder_guard.set_node_type(NodeType::NonshiftingPlaceholder);
                }
                place_node_at_index(&placeholder, layer, node_index + 1);
            }
            edge_count += 1;
        }

        extra_edge = true;
    }

    if edge_count > 0 {
        edge_count - 1
    } else {
        0
    }
}

fn create_dummy_node(graph_ref: &LGraphRef, edge_to_split: &LEdgeRef) -> LNodeRef {
    let dummy_node = LNode::new(graph_ref);
    if let Some(mut dummy_guard) = dummy_node.lock_ok() {
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
    dummy_node
}

fn place_node_at_index(node: &LNodeRef, layer: &LayerRef, index: usize) {
    let layer_size = layer
        .lock_ok()
        .map(|layer_guard| layer_guard.nodes().len())
        .unwrap_or(0);
    if index > layer_size {
        LNode::set_layer(node, Some(layer.clone()));
    } else {
        LNode::set_layer_at_index(node, index, Some(layer.clone()));
    }
}

fn remove_unconnected_placeholder_nodes(graph: &mut LGraph) {
    for layer in graph.layers().clone() {
        let nodes = layer
            .lock_ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            let node_type = node
                .lock_ok()
                .map(|node_guard| node_guard.node_type())
                .unwrap_or(NodeType::Normal);
            if node_type == NodeType::Placeholder || node_type == NodeType::NonshiftingPlaceholder {
                LNode::set_layer(&node, None);
            }
        }
    }
}

fn has_node_property<T: Clone + Send + Sync + 'static>(
    node: &LNodeRef,
    property: &Property<T>,
) -> bool {
    node.lock_ok()
        .map(|mut node_guard| {
            node_guard
                .shape()
                .graph_element()
                .properties()
                .has_property(property)
        })
        .unwrap_or(false)
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock_ok()
            .and_then(|layer_guard| layer_guard.graph())
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock_ok().and_then(|node_guard| node_guard.graph()) {
            return graph_ref;
        }
    }
    LGraph::new()
}
