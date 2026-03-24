use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::InvertedPortProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    {
        let mut graph_guard = graph.lock();        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_node(graph: &LGraphRef, layer: &LayerRef, constraints: PortConstraints) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(constraints));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    {
        let mut guard = port.lock();
        guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef) -> LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

fn run_processor(graph: &LGraphRef) {
    let mut processor = InvertedPortProcessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn inverted_port_processor_inserts_dummy_for_inverted_ports() {
    let (graph, layers) = graph_with_layers(2);
    let left_layer = layers[0].clone();
    let right_layer = layers[1].clone();

    let west_source_node = add_node(&graph, &left_layer, PortConstraints::FixedSide);
    let west_source = add_port(&west_source_node, PortSide::West);
    let west_target_node = add_node(&graph, &right_layer, PortConstraints::FixedSide);
    let west_target = add_port(&west_target_node, PortSide::West);
    connect(&west_source, &west_target);

    let east_source_node = add_node(&graph, &left_layer, PortConstraints::FixedSide);
    let east_source = add_port(&east_source_node, PortSide::East);
    let east_target_node = add_node(&graph, &right_layer, PortConstraints::FixedSide);
    let east_target = add_port(&east_target_node, PortSide::East);
    connect(&east_source, &east_target);

    run_processor(&graph);

    let west_first_edge = west_source
        .lock()
        
        .outgoing_edges()
        .first()
        .cloned()
        .expect("west source edge");
    let west_dummy = west_first_edge
        .lock()
        
        .target()
        .and_then(|port| port.lock().node())
        .expect("west dummy node");
    let west_dummy_type = west_dummy.lock().node_type();
    let west_dummy_layer = west_dummy
        .lock()
        
        .layer()
        .expect("west dummy layer");
    assert_eq!(west_dummy_type, NodeType::LongEdge);
    assert!(Arc::ptr_eq(&west_dummy_layer, &left_layer));

    let east_first_edge = east_target
        .lock()
        
        .incoming_edges()
        .first()
        .cloned()
        .expect("east target edge");
    let east_dummy = east_first_edge
        .lock()
        
        .source()
        .and_then(|port| port.lock().node())
        .expect("east dummy node");
    let east_dummy_type = east_dummy.lock().node_type();
    let east_dummy_layer = east_dummy
        .lock()
        
        .layer()
        .expect("east dummy layer");
    assert_eq!(east_dummy_type, NodeType::LongEdge);
    assert!(Arc::ptr_eq(&east_dummy_layer, &right_layer));
}

#[test]
fn inverted_port_processor_ignores_self_loop() {
    let (graph, layers) = graph_with_layers(1);
    let layer = layers[0].clone();
    let node = add_node(&graph, &layer, PortConstraints::FixedSide);
    let west = add_port(&node, PortSide::West);
    let east = add_port(&node, PortSide::East);
    connect(&west, &east);

    run_processor(&graph);

    let nodes = layer.lock().nodes().clone();
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        nodes[0].lock().node_type(),
        NodeType::Normal
    );
}

#[test]
fn inverted_port_processor_skips_dummy_for_same_layer_east_input() {
    let (graph, layers) = graph_with_layers(1);
    let layer = layers[0].clone();

    // Two nodes in the same layer, source has EAST output, target has EAST input
    let source_node = add_node(&graph, &layer, PortConstraints::FixedSide);
    let target_node = add_node(&graph, &layer, PortConstraints::FixedSide);
    let source_east = add_port(&source_node, PortSide::East);
    let target_east = add_port(&target_node, PortSide::East);
    connect(&source_east, &target_east);

    run_processor(&graph);

    // No dummy node should be created for same-layer EAST→EAST edge
    let nodes = layer.lock().nodes().clone();
    assert_eq!(nodes.len(), 2, "no dummy node should be added for same-layer edge");
}

#[test]
fn inverted_port_processor_skips_dummy_for_same_layer_west_output() {
    let (graph, layers) = graph_with_layers(1);
    let layer = layers[0].clone();

    // Two nodes in the same layer, source has WEST output, target has WEST input
    let source_node = add_node(&graph, &layer, PortConstraints::FixedSide);
    let target_node = add_node(&graph, &layer, PortConstraints::FixedSide);
    let source_west = add_port(&source_node, PortSide::West);
    let target_west = add_port(&target_node, PortSide::West);
    connect(&source_west, &target_west);

    run_processor(&graph);

    // No dummy node should be created for same-layer WEST→WEST edge
    let nodes = layer.lock().nodes().clone();
    assert_eq!(nodes.len(), 2, "no dummy node should be added for same-layer edge");
}

#[test]
fn inverted_port_processor_creates_dummy_for_cross_layer_east_input() {
    let (graph, layers) = graph_with_layers(2);
    let layer0 = layers[0].clone();
    let layer1 = layers[1].clone();

    // Source in layer0, target in layer1 with EAST input — should still create dummy
    let source_node = add_node(&graph, &layer0, PortConstraints::FixedSide);
    let target_node = add_node(&graph, &layer1, PortConstraints::FixedSide);
    let source_east = add_port(&source_node, PortSide::East);
    let target_east = add_port(&target_node, PortSide::East);
    connect(&source_east, &target_east);

    run_processor(&graph);

    // Dummy should be created for cross-layer inverted port
    let nodes = layer1.lock().nodes().clone();
    assert!(nodes.len() > 1, "dummy node should be created for cross-layer inverted port");
}
