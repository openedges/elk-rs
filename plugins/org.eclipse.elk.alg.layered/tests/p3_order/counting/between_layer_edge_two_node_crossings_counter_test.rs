#![allow(dead_code)]

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::greedyswitch::BetweenLayerEdgeTwoNodeCrossingsCounter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

fn new_graph() -> LGraphRef {
    let graph = LGraph::new();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
        graph_guard.set_property(InternalProperties::RANDOM, Some(Random::new(0)));
    }
    graph
}

fn make_layer(graph: &LGraphRef) -> LayerRef {
    let layer = Layer::new(graph);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }
    layer
}

fn add_node_to_layer(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_nodes_to_layer(graph: &LGraphRef, layer: &LayerRef, count: usize) -> Vec<LNodeRef> {
    (0..count)
        .map(|_| add_node_to_layer(graph, layer))
        .collect()
}

fn add_port_on_side(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    if let Ok(mut port_guard) = port.lock() {
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn add_edge_between_ports(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn east_west_edge_from_to(left: &LNodeRef, right: &LNodeRef) {
    let left_port = add_port_on_side(left, PortSide::East);
    let right_port = add_port_on_side(right, PortSide::West);
    add_edge_between_ports(&left_port, &right_port);
}

fn east_west_edge_from_port(left_port: &LPortRef, right: &LNodeRef) {
    let right_port = add_port_on_side(right, PortSide::West);
    add_edge_between_ports(left_port, &right_port);
}

fn east_west_edge_from_node(left: &LNodeRef, right_port: &LPortRef) {
    let left_port = add_port_on_side(left, PortSide::East);
    add_edge_between_ports(&left_port, right_port);
}

fn add_in_layer_edge(node_one: &LNodeRef, node_two: &LNodeRef, side: PortSide) {
    let port_one = add_port_on_side(node_one, side);
    let port_two = add_port_on_side(node_two, side);
    add_edge_between_ports(&port_one, &port_two);
}

fn self_loop_on(node: &LNodeRef, side: PortSide) {
    let port = add_port_on_side(node, side);
    let target = add_port_on_side(node, side);
    add_edge_between_ports(&port, &target);
}

fn set_fixed_order_constraint(node: &LNodeRef) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedOrder),
        );
        if let Some(graph) = node_guard.graph() {
            if let Ok(mut graph_guard) = graph.lock() {
                let mut props = graph_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of);
                props.insert(GraphProperties::NonFreePorts);
                graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
            }
        }
    }
}

fn graph_two_nodes_no_connection() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    add_node_to_layer(&graph, &layer);
    add_node_to_layer(&graph, &layer);
    graph
}

fn graph_cross_formed() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    graph
}

fn graph_one_node() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    add_node_to_layer(&graph, &layer);
    graph
}

fn graph_multiple_edges_between_same_nodes() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    graph
}

fn graph_cross_with_extra_edge_in_between() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    east_west_edge_from_to(&left_nodes[0], &right_nodes[2]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[2], &right_nodes[0]);
    graph
}

fn graph_in_layer_edges() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    east_west_edge_from_to(&middle_nodes[1], &right_node);
    east_west_edge_from_to(&left_node, &middle_nodes[1]);
    add_in_layer_edge(&middle_nodes[0], &middle_nodes[2], PortSide::West);
    graph
}

fn graph_in_layer_edges_crossings_when_switched() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    add_in_layer_edge(&right_nodes[0], &right_nodes[1], PortSide::West);
    east_west_edge_from_to(&left_node, &right_nodes[2]);
    graph
}

fn graph_cross_with_many_self_loops() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    let top_left_port = add_port_on_side(&left_nodes[0], PortSide::East);
    let bottom_left_port = add_port_on_side(&left_nodes[1], PortSide::East);

    for layer in graph.lock().expect("graph lock").layers().clone() {
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            self_loop_on(&node, PortSide::East);
            self_loop_on(&node, PortSide::East);
            self_loop_on(&node, PortSide::East);
            self_loop_on(&node, PortSide::West);
            self_loop_on(&node, PortSide::West);
            self_loop_on(&node, PortSide::West);
        }
    }

    let top_right_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let bottom_right_port = add_port_on_side(&right_nodes[1], PortSide::West);
    add_edge_between_ports(&top_left_port, &bottom_right_port);
    add_edge_between_ports(&bottom_left_port, &top_right_port);
    graph
}

fn graph_more_complex_three_layer() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 3);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    let left_middle_port = add_port_on_side(&left_nodes[1], PortSide::East);
    let middle_lower_east = add_port_on_side(&middle_nodes[1], PortSide::East);
    let middle_upper_east = add_port_on_side(&middle_nodes[0], PortSide::East);
    let right_upper = add_port_on_side(&right_nodes[0], PortSide::West);
    let right_middle = add_port_on_side(&right_nodes[1], PortSide::West);

    add_edge_between_ports(&middle_upper_east, &right_upper);
    add_edge_between_ports(&middle_upper_east, &right_middle);
    add_edge_between_ports(&middle_upper_east, &right_middle);
    east_west_edge_from_port(&middle_lower_east, &right_nodes[2]);
    east_west_edge_from_port(&left_middle_port, &middle_nodes[0]);
    east_west_edge_from_node(&middle_nodes[1], &right_upper);
    east_west_edge_from_port(&left_middle_port, &middle_nodes[1]);
    east_west_edge_from_to(&left_nodes[2], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);
    graph
}

fn graph_fixed_port_order() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    set_fixed_order_constraint(&left_node);

    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);
    east_west_edge_from_to(&left_node, &right_nodes[1]);
    east_west_edge_from_to(&left_node, &right_nodes[0]);
    graph
}

fn graph_two_edges_into_same_port() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    let bottom_left_first = add_port_on_side(&left_nodes[1], PortSide::East);
    let bottom_left_second = add_port_on_side(&left_nodes[1], PortSide::East);
    let top_right_first = add_port_on_side(&right_nodes[0], PortSide::West);
    let top_right_second = add_port_on_side(&right_nodes[0], PortSide::West);

    add_edge_between_ports(&bottom_left_first, &top_right_first);
    add_edge_between_ports(&bottom_left_second, &top_right_second);
    graph
}

fn graph_two_edges_into_same_port_crosses_when_switched() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    let top_right_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let bottom_left_port = add_port_on_side(&left_nodes[1], PortSide::East);
    add_edge_between_ports(&bottom_left_port, &top_right_port);

    let top_left_port = add_port_on_side(&left_nodes[0], PortSide::East);
    add_edge_between_ports(&top_left_port, &top_right_port);

    east_west_edge_from_to(&left_nodes[1], &right_nodes[1]);
    graph
}

fn graph_two_edges_into_same_port_resolves_crossing_when_switched() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    let top_left_port = add_port_on_side(&left_nodes[0], PortSide::East);
    let bottom_left_port = add_port_on_side(&left_nodes[1], PortSide::East);
    let bottom_right_port = add_port_on_side(&right_nodes[1], PortSide::West);

    add_edge_between_ports(&top_left_port, &bottom_right_port);
    add_edge_between_ports(&bottom_left_port, &bottom_right_port);

    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    graph
}

fn graph_two_edges_into_same_port_from_east_with_fixed_port_order() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&left_nodes[1]);
    set_fixed_order_constraint(&right_nodes[0]);

    let top_left_port = add_port_on_side(&left_nodes[0], PortSide::East);
    let bottom_left_port = add_port_on_side(&left_nodes[1], PortSide::East);
    let top_right_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let bottom_right_port = add_port_on_side(&right_nodes[1], PortSide::West);

    add_edge_between_ports(&bottom_left_port, &bottom_right_port);
    add_edge_between_ports(&bottom_left_port, &top_right_port);
    add_edge_between_ports(&top_left_port, &top_right_port);
    graph
}

fn setup_counter(
    graph: &LGraphRef,
    free_layer_index: usize,
    upper_index: usize,
    lower_index: usize,
) -> (BetweenLayerEdgeTwoNodeCrossingsCounter, LNodeRef, LNodeRef) {
    let node_order = graph.lock().expect("graph lock").to_node_array();
    let layer_nodes = node_order
        .get(free_layer_index)
        .cloned()
        .expect("free layer");
    let upper = layer_nodes[upper_index].clone();
    let lower = layer_nodes[lower_index].clone();
    (
        BetweenLayerEdgeTwoNodeCrossingsCounter::new(node_order, free_layer_index),
        upper,
        lower,
    )
}

fn assert_eastern(
    counter: &mut BetweenLayerEdgeTwoNodeCrossingsCounter,
    upper: &LNodeRef,
    lower: &LNodeRef,
    expected_upper_lower: i32,
    expected_lower_upper: i32,
) {
    counter.count_eastern_edge_crossings(upper, lower);
    assert_eq!(counter.upper_lower_crossings(), expected_upper_lower);
    assert_eq!(counter.lower_upper_crossings(), expected_lower_upper);
}

fn assert_western(
    counter: &mut BetweenLayerEdgeTwoNodeCrossingsCounter,
    upper: &LNodeRef,
    lower: &LNodeRef,
    expected_upper_lower: i32,
    expected_lower_upper: i32,
) {
    counter.count_western_edge_crossings(upper, lower);
    assert_eq!(counter.upper_lower_crossings(), expected_upper_lower);
    assert_eq!(counter.lower_upper_crossings(), expected_lower_upper);
}

fn assert_both(
    counter: &mut BetweenLayerEdgeTwoNodeCrossingsCounter,
    upper: &LNodeRef,
    lower: &LNodeRef,
    expected_upper_lower: i32,
    expected_lower_upper: i32,
) {
    counter.count_both_side_crossings(upper, lower);
    assert_eq!(counter.upper_lower_crossings(), expected_upper_lower);
    assert_eq!(counter.lower_upper_crossings(), expected_lower_upper);
}

#[test]
fn two_node_no_edges() {
    let graph = graph_two_nodes_no_connection();
    let (mut counter, upper, lower) = setup_counter(&graph, 0, 0, 1);
    assert_both(&mut counter, &upper, &lower, 0, 0);
    assert_western(&mut counter, &upper, &lower, 0, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}

#[test]
fn cross_formed() {
    let graph = graph_cross_formed();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 1);
    assert_both(&mut counter, &upper, &lower, 1, 0);
    assert_western(&mut counter, &upper, &lower, 1, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}

#[test]
fn one_node() {
    let graph = graph_one_node();
    let (mut counter, upper, _lower) = setup_counter(&graph, 0, 0, 0);
    assert_both(&mut counter, &upper, &upper, 0, 0);
    assert_western(&mut counter, &upper, &upper, 0, 0);
    assert_eastern(&mut counter, &upper, &upper, 0, 0);
}

#[test]
fn cross_formed_multiple_edges_between_same_nodes() {
    let graph = graph_multiple_edges_between_same_nodes();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 1);
    assert_both(&mut counter, &upper, &lower, 4, 0);
    assert_western(&mut counter, &upper, &lower, 4, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}

#[test]
fn cross_with_extra_edge_in_between() {
    let graph = graph_cross_with_extra_edge_in_between();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 2);
    assert_both(&mut counter, &upper, &lower, 1, 0);
    assert_western(&mut counter, &upper, &lower, 1, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}

#[test]
fn ignore_in_layer_edges() {
    let graph = graph_in_layer_edges();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 2);
    assert_both(&mut counter, &upper, &lower, 0, 0);
    assert_western(&mut counter, &upper, &lower, 0, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}

#[test]
fn ignore_self_loops() {
    let graph = graph_cross_with_many_self_loops();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 1);
    assert_both(&mut counter, &upper, &lower, 1, 0);
    assert_western(&mut counter, &upper, &lower, 1, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}

#[test]
fn more_complex_three_layer_graph() {
    let graph = graph_more_complex_three_layer();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 1);
    assert_western(&mut counter, &upper, &lower, 1, 1);
    assert_eastern(&mut counter, &upper, &lower, 2, 3);
    assert_both(&mut counter, &upper, &lower, 3, 4);
}

#[test]
fn fixed_port_order() {
    let graph = graph_fixed_port_order();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
    assert_western(&mut counter, &upper, &lower, 1, 0);
    assert_both(&mut counter, &upper, &lower, 1, 0);
}

#[test]
fn switch_three_times() {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 4);
    let left_top_port = add_port_on_side(&left_nodes[0], PortSide::East);
    let left_lower_port = add_port_on_side(&left_nodes[1], PortSide::East);
    let right_top_port = add_port_on_side(&right_nodes[0], PortSide::West);

    add_edge_between_ports(&left_lower_port, &right_top_port);
    east_west_edge_from_port(&left_lower_port, &right_nodes[2]);

    add_edge_between_ports(&left_top_port, &right_top_port);
    east_west_edge_from_port(&left_top_port, &right_nodes[1]);
    east_west_edge_from_port(&left_top_port, &right_nodes[3]);

    let (mut counter, upper, lower) = setup_counter(&graph, 0, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 3, 2);
    assert_western(&mut counter, &upper, &lower, 0, 0);
    assert_both(&mut counter, &upper, &lower, 3, 2);
}

#[test]
fn into_same_port() {
    let graph = graph_two_edges_into_same_port();
    let (mut counter, upper, lower) = setup_counter(&graph, 1, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
    assert_western(&mut counter, &upper, &lower, 2, 0);
    assert_both(&mut counter, &upper, &lower, 2, 0);
}

#[test]
fn into_same_port_causes_crossings_on_switch() {
    let graph = graph_two_edges_into_same_port_crosses_when_switched();
    let (mut counter, upper, lower) = setup_counter(&graph, 0, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 0, 1);
    assert_western(&mut counter, &upper, &lower, 0, 0);
    assert_both(&mut counter, &upper, &lower, 0, 1);
}

#[test]
fn into_same_port_reduces_crossings_on_switch() {
    let graph = graph_two_edges_into_same_port_resolves_crossing_when_switched();
    let (mut counter, upper, lower) = setup_counter(&graph, 0, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 1, 0);
    assert_western(&mut counter, &upper, &lower, 0, 0);
    assert_both(&mut counter, &upper, &lower, 1, 0);
}

#[test]
fn into_same_port_from_east_switch_with_fixed_port_order() {
    let graph = graph_two_edges_into_same_port_from_east_with_fixed_port_order();
    let (mut counter, upper, lower) = setup_counter(&graph, 0, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 0, 1);
    assert_western(&mut counter, &upper, &lower, 0, 0);
    assert_both(&mut counter, &upper, &lower, 0, 1);
}

#[test]
fn multiple_edges_into_same_port_causes_no_crossings() {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let top_left = add_node_to_layer(&graph, &left_layer);
    let bottom_left = add_node_to_layer(&graph, &left_layer);
    let bottom_right = add_node_to_layer(&graph, &right_layer);

    let bottom_right_port = add_port_on_side(&bottom_right, PortSide::West);
    east_west_edge_from_node(&top_left, &bottom_right_port);
    east_west_edge_from_node(&top_left, &bottom_right_port);
    east_west_edge_from_node(&bottom_left, &bottom_right_port);

    let (mut counter, upper, lower) = setup_counter(&graph, 0, 0, 1);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
    assert_eastern(&mut counter, &upper, &lower, 0, 0);
}
