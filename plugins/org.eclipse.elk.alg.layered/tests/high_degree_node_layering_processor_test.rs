mod issue_support;

use std::sync::Arc;

use issue_support::init_layered_options;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::HighDegreeNodeLayeringProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    if let Ok(mut graph_guard) = graph.lock() {
        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn connect(source_node: &LNodeRef, target_node: &LNodeRef) {
    let source_port = LPort::new();
    if let Ok(mut source_guard) = source_port.lock() {
        source_guard.set_side(PortSide::East);
    }
    LPort::set_node(&source_port, Some(source_node.clone()));

    let target_port = LPort::new();
    if let Ok(mut target_guard) = target_port.lock() {
        target_guard.set_side(PortSide::West);
    }
    LPort::set_node(&target_port, Some(target_node.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port));
    LEdge::set_target(&edge, Some(target_port));
}

fn node_layer(node: &LNodeRef) -> LayerRef {
    node.lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .expect("node layer")
}

fn node_layer_index(graph: &LGraphRef, node: &LNodeRef) -> usize {
    let layer = node_layer(node);
    graph
        .lock()
        .ok()
        .and_then(|graph_guard| {
            graph_guard
                .layers()
                .iter()
                .position(|candidate| Arc::ptr_eq(candidate, &layer))
        })
        .expect("layer index")
}

#[test]
fn moves_incoming_and_outgoing_leaf_trees_to_inserted_layers() {
    init_layered_options();

    let (graph, layers) = new_graph_with_layers(3);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::HIGH_DEGREE_NODES_THRESHOLD, Some(2));
        graph_guard.set_property(LayeredOptions::HIGH_DEGREE_NODES_TREE_HEIGHT, Some(5));
    }

    let incoming_root = add_node(&graph, &layers[0]);
    let high_degree = add_node(&graph, &layers[1]);
    let outgoing_root = add_node(&graph, &layers[2]);

    connect(&incoming_root, &high_degree);
    connect(&high_degree, &outgoing_root);

    let incoming_before = layers[0].clone();
    let outgoing_before = layers[2].clone();

    let mut processor = HighDegreeNodeLayeringProcessor::default();
    let mut monitor = NullElkProgressMonitor;
    if let Ok(mut graph_guard) = graph.lock() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    let incoming_after = node_layer(&incoming_root);
    let outgoing_after = node_layer(&outgoing_root);
    assert!(!Arc::ptr_eq(&incoming_after, &incoming_before));
    assert!(!Arc::ptr_eq(&outgoing_after, &outgoing_before));
    assert!(Arc::ptr_eq(&node_layer(&high_degree), &layers[1]));

    let layer_count = graph
        .lock()
        .ok()
        .map(|g| g.layers().len())
        .unwrap_or_default();
    assert_eq!(layer_count, 3);
    assert_eq!(node_layer_index(&graph, &incoming_root), 0);
    assert_eq!(node_layer_index(&graph, &high_degree), 1);
    assert_eq!(node_layer_index(&graph, &outgoing_root), 2);
}

#[test]
fn tree_height_zero_is_treated_as_unbounded() {
    init_layered_options();

    let (graph, layers) = new_graph_with_layers(3);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::HIGH_DEGREE_NODES_THRESHOLD, Some(3));
        graph_guard.set_property(LayeredOptions::HIGH_DEGREE_NODES_TREE_HEIGHT, Some(0));
    }

    let leaf = add_node(&graph, &layers[0]);
    let root = add_node(&graph, &layers[1]);
    let aux_a = add_node(&graph, &layers[1]);
    let aux_b = add_node(&graph, &layers[1]);
    let high_degree = add_node(&graph, &layers[2]);

    connect(&leaf, &root);
    connect(&root, &high_degree);
    connect(&aux_a, &high_degree);
    connect(&aux_b, &high_degree);

    let leaf_before = layers[0].clone();
    let root_before = layers[1].clone();

    let mut processor = HighDegreeNodeLayeringProcessor::default();
    let mut monitor = NullElkProgressMonitor;
    if let Ok(mut graph_guard) = graph.lock() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    let leaf_after = node_layer(&leaf);
    let root_after = node_layer(&root);
    assert!(!Arc::ptr_eq(&leaf_after, &leaf_before));
    assert!(!Arc::ptr_eq(&root_after, &root_before));

    let high_degree_index = node_layer_index(&graph, &high_degree);
    let root_index = node_layer_index(&graph, &root);
    let leaf_index = node_layer_index(&graph, &leaf);
    assert_eq!(high_degree_index, root_index + 1);
    assert_eq!(root_index, leaf_index + 1);
}
