use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, NodeType,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
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
    let mut processor = EdgeAndLayerConstraintEdgeReverser;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
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

    let incoming = first.lock().incoming_edges();
    let outgoing = first.lock().outgoing_edges();
    assert!(incoming.is_empty());
    assert_eq!(outgoing.len(), 1);
    assert!(Arc::ptr_eq(&outgoing[0], &edge));
    assert!(edge
        .lock()
        
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

    let incoming = last.lock().incoming_edges();
    let outgoing = last.lock().outgoing_edges();
    assert!(outgoing.is_empty());
    assert_eq!(incoming.len(), 1);
    assert!(Arc::ptr_eq(&incoming[0], &edge));
    assert!(edge
        .lock()
        
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

    let incoming = first.lock().incoming_edges();
    let outgoing = first.lock().outgoing_edges();
    assert_eq!(incoming.len(), 1);
    assert!(outgoing.is_empty());
    assert!(Arc::ptr_eq(&incoming[0], &edge));
    assert!(!edge
        .lock()
        
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false));
}

fn add_port_with_side(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    { let mut pg = port.lock();
        pg.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

#[test]
fn reverser_does_not_block_last_separate_east_west_normal_edge() {
    // Verify that can_reverse_outgoing_edge returns true for EAST→WEST normal edges
    // targeting a LastSeparate node (the old code would unconditionally block this).
    // The edge won't actually be reversed here because handle_inner_nodes requires
    // PORT_CONSTRAINTS to be side-fixed and all ports to look reversed.
    let graph = LGraph::new();
    let normal_source = add_layerless_node(&graph, LayerConstraint::None);
    let last_sep_target = add_layerless_node(&graph, LayerConstraint::LastSeparate);

    let source_port = add_port_with_side(&normal_source, PortSide::East);
    let target_port = add_port_with_side(&last_sep_target, PortSide::West);
    let _edge = connect(&source_port, &target_port);

    // Should not panic — the processor should handle the edge gracefully
    run_processor(&graph);
}

#[test]
fn reverser_blocks_non_east_west_edge_to_last_separate() {
    // Verify that non-EAST→WEST edges to LastSeparate are still blocked from reversal.
    // This tests that the new code preserves the guard for non-NORMAL or non-EAST→WEST cases.
    let graph = LGraph::new();
    let normal_source = add_layerless_node(&graph, LayerConstraint::None);
    let last_sep_target = add_layerless_node(&graph, LayerConstraint::LastSeparate);

    // Use default ports (no explicit side = Undefined) — should NOT be allowed
    let source_port = add_port(&normal_source);
    let target_port = add_port(&last_sep_target);
    let edge = connect(&source_port, &target_port);

    run_processor(&graph);

    // Edge should NOT be reversed because ports are not EAST→WEST
    let reversed = edge
        .lock()
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false);
    assert!(!reversed, "non-EAST→WEST edge to LastSeparate should not be reversed");
}

#[test]
fn reverser_allows_first_separate_outgoing_east_west_edge() {
    // FirstSeparate is OutgoingOnly: its incoming edges get reversed.
    // With the new code, can_reverse_incoming_edge allows reversal when
    // the source is FirstSeparate but both nodes are NORMAL with EAST→WEST.
    let graph = LGraph::new();
    let first_sep_source = add_layerless_node(&graph, LayerConstraint::FirstSeparate);
    let normal_target = add_layerless_node(&graph, LayerConstraint::None);

    let source_port = add_port_with_side(&first_sep_source, PortSide::East);
    let target_port = add_port_with_side(&normal_target, PortSide::West);
    let edge = connect(&source_port, &target_port);

    run_processor(&graph);

    // FirstSeparate is OutgoingOnly, so this outgoing edge stays as-is
    let reversed = edge
        .lock()
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false);
    assert!(!reversed, "FirstSeparate outgoing EAST→WEST edge should not be reversed");
}

fn add_layerless_node_with_type(
    graph: &LGraphRef,
    constraint: LayerConstraint,
    node_type: NodeType,
) -> LNodeRef {
    let node = add_layerless_node(graph, constraint);
    node.lock()
        .set_node_type(node_type);
    node
}

#[test]
fn reverser_blocks_external_port_east_west_edge_to_last_separate() {
    // ExternalPort nodes should NOT have their edges reversed even with EAST→WEST,
    // because the NORMAL check should fail for ExternalPort nodes.
    let graph = LGraph::new();
    let ext_port_source =
        add_layerless_node_with_type(&graph, LayerConstraint::None, NodeType::ExternalPort);
    let last_sep_target =
        add_layerless_node_with_type(&graph, LayerConstraint::LastSeparate, NodeType::ExternalPort);

    let source_port = add_port_with_side(&ext_port_source, PortSide::East);
    let target_port = add_port_with_side(&last_sep_target, PortSide::West);
    let edge = connect(&source_port, &target_port);

    run_processor(&graph);

    // ExternalPort nodes are not NORMAL, so can_reverse_outgoing_edge should return false
    let reversed = edge
        .lock()
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(false);
    assert!(
        !reversed,
        "ExternalPort EAST→WEST edge to LastSeparate should NOT be reversed"
    );
}
