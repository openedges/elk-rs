use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LayerConstraintPreprocessor;
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
            
            .set_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT, Some(constraint));
    }
    graph
        .lock()
        
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
    let mut processor = LayerConstraintPreprocessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn layer_constraint_preprocessor_hides_first_separate_and_remembers_opposite_port() {
    let graph = LGraph::new();
    let hidden = add_layerless_node(&graph, LayerConstraint::FirstSeparate);
    let opposite = add_layerless_node(&graph, LayerConstraint::None);

    let hidden_port = add_port(&hidden);
    let opposite_port = add_port(&opposite);
    let edge = connect(&hidden_port, &opposite_port);

    run_processor(&graph);

    let layerless_nodes = graph.lock().layerless_nodes().clone();
    assert_eq!(layerless_nodes.len(), 1);
    assert!(Arc::ptr_eq(&layerless_nodes[0], &opposite));

    let hidden_nodes = graph
        .lock()
        
        .get_property(InternalProperties::HIDDEN_NODES)
        .unwrap_or_default();
    assert_eq!(hidden_nodes.len(), 1);
    assert!(Arc::ptr_eq(&hidden_nodes[0], &hidden));

    assert!(edge.lock().target().is_none());
    assert!(edge
        .lock()
        
        .get_property(InternalProperties::ORIGINAL_OPPOSITE_PORT)
        .is_some_and(|port| Arc::ptr_eq(&port, &opposite_port)));

    let assigned = opposite
        .lock()
        
        .get_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT)
        .unwrap_or(LayerConstraint::None);
    assert_eq!(assigned, LayerConstraint::First);
}

#[test]
fn layer_constraint_preprocessor_does_not_force_layer_when_connected_to_both_hidden_sides() {
    let graph = LGraph::new();
    let first_hidden = add_layerless_node(&graph, LayerConstraint::FirstSeparate);
    let last_hidden = add_layerless_node(&graph, LayerConstraint::LastSeparate);
    let opposite = add_layerless_node(&graph, LayerConstraint::None);

    let first_port = add_port(&first_hidden);
    let last_port = add_port(&last_hidden);
    let opposite_port_a = add_port(&opposite);
    let opposite_port_b = add_port(&opposite);
    let _edge_a = connect(&first_port, &opposite_port_a);
    let _edge_b = connect(&opposite_port_b, &last_port);

    run_processor(&graph);

    let has_constraint = opposite
        .lock()
        
        .shape()
        .graph_element()
        .properties()
        .has_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT);
    assert!(!has_constraint);
}

#[test]
fn preprocessor_does_not_panic_with_incoming_edge_on_first_separate() {
    // With USE_ENSURE_NO_INACCEPTABLE_EDGES=false, this should not panic
    let graph = LGraph::new();
    let normal = add_layerless_node(&graph, LayerConstraint::None);
    let first_sep = add_layerless_node(&graph, LayerConstraint::FirstSeparate);

    let normal_port = add_port(&normal);
    let first_sep_port = add_port(&first_sep);
    // incoming edge to FIRST_SEPARATE (would have panicked before)
    let _edge = connect(&normal_port, &first_sep_port);

    // Should not panic
    run_processor(&graph);

    let hidden_nodes = graph
        .lock()
        .expect("graph lock")
        .get_property(InternalProperties::HIDDEN_NODES)
        .unwrap_or_default();
    assert!(
        hidden_nodes.iter().any(|n| Arc::ptr_eq(n, &first_sep)),
        "first_separate node should be hidden"
    );
}

#[test]
fn preprocessor_does_not_panic_with_outgoing_edge_on_last_separate() {
    // With USE_ENSURE_NO_INACCEPTABLE_EDGES=false, this should not panic
    let graph = LGraph::new();
    let last_sep = add_layerless_node(&graph, LayerConstraint::LastSeparate);
    let normal = add_layerless_node(&graph, LayerConstraint::None);

    let last_sep_port = add_port(&last_sep);
    let normal_port = add_port(&normal);
    // outgoing edge from LAST_SEPARATE (would have panicked before)
    let _edge = connect(&last_sep_port, &normal_port);

    // Should not panic
    run_processor(&graph);

    let hidden_nodes = graph
        .lock()
        .expect("graph lock")
        .get_property(InternalProperties::HIDDEN_NODES)
        .unwrap_or_default();
    assert!(
        hidden_nodes.iter().any(|n| Arc::ptr_eq(n, &last_sep)),
        "last_separate node should be hidden"
    );
}
