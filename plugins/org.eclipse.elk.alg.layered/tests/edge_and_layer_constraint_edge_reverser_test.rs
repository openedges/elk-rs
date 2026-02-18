use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::EdgeAndLayerConstraintEdgeReverser;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn add_layerless_node(graph: &LGraphRef, constraint: LayerConstraint) -> LNodeRef {
    let node = LNode::new(graph);
    if constraint != LayerConstraint::None {
        node.lock()
            .expect("node lock")
            .set_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT, Some(constraint));
    }
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(node.clone());
    node
}

fn add_port(node: &LNodeRef) -> LPortRef {
    let port = LPort::new();
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
    LayoutMetaDataService::get_instance()
        .register_layout_meta_data_provider(&LayeredMetaDataProvider);
    let mut processor = EdgeAndLayerConstraintEdgeReverser;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock().expect("graph lock");
    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn reverser_makes_first_node_outgoing_only() {
    let graph = LGraph::new();
    let source = add_layerless_node(&graph, LayerConstraint::None);
    let first = add_layerless_node(&graph, LayerConstraint::First);

    let source_port = add_port(&source);
    let first_port = add_port(&first);
    let edge = connect(&source_port, &first_port);

    run_processor(&graph);

    let incoming = first.lock().expect("first lock").incoming_edges();
    let outgoing = first.lock().expect("first lock").outgoing_edges();
    assert!(incoming.is_empty());
    assert_eq!(outgoing.len(), 1);
    assert!(Arc::ptr_eq(&outgoing[0], &edge));
    assert!(edge
        .lock()
        .expect("edge lock")
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false));
}

#[test]
fn reverser_makes_last_node_incoming_only() {
    let graph = LGraph::new();
    let last = add_layerless_node(&graph, LayerConstraint::Last);
    let target = add_layerless_node(&graph, LayerConstraint::None);

    let last_port = add_port(&last);
    let target_port = add_port(&target);
    let edge = connect(&last_port, &target_port);

    run_processor(&graph);

    let incoming = last.lock().expect("last lock").incoming_edges();
    let outgoing = last.lock().expect("last lock").outgoing_edges();
    assert!(outgoing.is_empty());
    assert_eq!(incoming.len(), 1);
    assert!(Arc::ptr_eq(&incoming[0], &edge));
    assert!(edge
        .lock()
        .expect("edge lock")
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false));
}

#[test]
fn reverser_keeps_first_separate_to_first_edge_direction() {
    let graph = LGraph::new();
    let first_separate = add_layerless_node(&graph, LayerConstraint::FirstSeparate);
    let first = add_layerless_node(&graph, LayerConstraint::First);

    let source_port = add_port(&first_separate);
    let target_port = add_port(&first);
    let edge = connect(&source_port, &target_port);

    run_processor(&graph);

    let incoming = first.lock().expect("first lock").incoming_edges();
    let outgoing = first.lock().expect("first lock").outgoing_edges();
    assert_eq!(incoming.len(), 1);
    assert!(outgoing.is_empty());
    assert!(Arc::ptr_eq(&incoming[0], &edge));
    assert!(!edge
        .lock()
        .expect("edge lock")
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false));
}
