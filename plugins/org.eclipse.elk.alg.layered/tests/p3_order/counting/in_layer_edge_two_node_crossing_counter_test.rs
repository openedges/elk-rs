#![allow(dead_code)]

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::CrossingsCounter;
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

fn make_layers(graph: &LGraphRef, count: usize) -> Vec<LayerRef> {
    (0..count).map(|_| make_layer(graph)).collect()
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

fn assign_ids(graph: &LGraphRef) -> usize {
    let mut port_id = 0usize;
    if let Ok(graph_guard) = graph.lock() {
        for (layer_idx, layer) in graph_guard.layers().iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_idx as i32;
                for (node_idx, node) in layer_guard.nodes().iter().enumerate() {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.shape().graph_element().id = node_idx as i32;
                        for port in node_guard.ports_mut() {
                            if let Ok(mut port_guard) = port.lock() {
                                port_guard.shape().graph_element().id = port_id as i32;
                            }
                            port_id += 1;
                        }
                    }
                }
            }
        }
    }
    port_id
}

struct InLayerCrossingHarness {
    node_order: Vec<LNodeRef>,
    left_counter: CrossingsCounter,
    right_counter: CrossingsCounter,
    upper_lower: i32,
    lower_upper: i32,
}

impl InLayerCrossingHarness {
    fn new(graph: &LGraphRef, layer_index: usize) -> Self {
        let total_ports = assign_ids(graph);
        let node_order = graph
            .lock()
            .expect("graph lock")
            .to_node_array()
            .get(layer_index)
            .cloned()
            .expect("layer index");

        for (idx, node) in node_order.iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.shape().graph_element().id = idx as i32;
            }
        }

        let mut left_counter = CrossingsCounter::new(vec![0; total_ports]);
        let mut right_counter = CrossingsCounter::new(vec![0; total_ports]);
        left_counter.init_port_positions_for_in_layer_crossings(&node_order, PortSide::West);
        right_counter.init_port_positions_for_in_layer_crossings(&node_order, PortSide::East);

        InLayerCrossingHarness {
            node_order,
            left_counter,
            right_counter,
            upper_lower: 0,
            lower_upper: 0,
        }
    }

    fn count_crossings(&mut self, upper_index: usize, lower_index: usize) {
        let upper = self.node_order[upper_index].clone();
        let lower = self.node_order[lower_index].clone();
        let left = self
            .left_counter
            .count_in_layer_crossings_between_nodes_in_both_orders(&upper, &lower, PortSide::West);
        let right = self
            .right_counter
            .count_in_layer_crossings_between_nodes_in_both_orders(&upper, &lower, PortSide::East);
        self.upper_lower = left.first + right.first;
        self.lower_upper = left.second + right.second;
    }

    fn switch_order(&mut self, index_one: usize, index_two: usize) {
        let upper = self.node_order[index_one].clone();
        let lower = self.node_order[index_two].clone();
        self.left_counter
            .switch_nodes(&upper, &lower, PortSide::West);
        self.right_counter
            .switch_nodes(&upper, &lower, PortSide::East);
        self.node_order.swap(index_one, index_two);
    }
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

fn graph_in_layer_edges_with_crossings_to_between_layer_edge_with_fixed_port_order() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 2);
    let left_node = add_node_to_layer(&graph, &layers[0]);
    let right_nodes = add_nodes_to_layer(&graph, &layers[1], 2);

    set_fixed_order_constraint(&right_nodes[0]);

    east_west_edge_from_to(&left_node, &right_nodes[0]);
    add_in_layer_edge(&right_nodes[0], &right_nodes[1], PortSide::West);
    east_west_edge_from_to(&left_node, &right_nodes[0]);
    east_west_edge_from_to(&left_node, &right_nodes[0]);
    graph
}

fn graph_in_layer_edges_with_fixed_port_order_and_normal_edge_crossings() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 2);
    let left_node = add_node_to_layer(&graph, &layers[0]);
    let right_nodes = add_nodes_to_layer(&graph, &layers[1], 3);

    set_fixed_order_constraint(&right_nodes[0]);
    east_west_edge_from_to(&left_node, &right_nodes[0]);
    add_in_layer_edge(&right_nodes[0], &right_nodes[2], PortSide::West);
    east_west_edge_from_to(&left_node, &right_nodes[1]);
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

fn graph_in_layer_crossings_on_both_sides() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);
    let left_node = add_node_to_layer(&graph, &layers[0]);
    let middle_nodes = add_nodes_to_layer(&graph, &layers[1], 3);
    let right_node = add_node_to_layer(&graph, &layers[2]);

    add_in_layer_edge(&middle_nodes[0], &middle_nodes[2], PortSide::East);
    add_in_layer_edge(&middle_nodes[0], &middle_nodes[2], PortSide::West);
    east_west_edge_from_to(&middle_nodes[1], &right_node);
    east_west_edge_from_to(&left_node, &middle_nodes[1]);
    graph
}

fn graph_fixed_port_order_in_layer_edges_dont_cross_each_other() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 2);
    set_fixed_order_constraint(&nodes[0]);
    set_fixed_order_constraint(&nodes[1]);

    let upper_port_upper_node = add_port_on_side(&nodes[0], PortSide::East);
    let lower_port_upper_node = add_port_on_side(&nodes[0], PortSide::East);
    let upper_port_lower_node = add_port_on_side(&nodes[1], PortSide::East);
    let lower_port_lower_node = add_port_on_side(&nodes[1], PortSide::East);
    add_edge_between_ports(&upper_port_upper_node, &lower_port_lower_node);
    add_edge_between_ports(&lower_port_upper_node, &upper_port_lower_node);
    graph
}

fn graph_fixed_port_order_in_layer_edges_with_crossings() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 2);
    set_fixed_order_constraint(&nodes[0]);
    set_fixed_order_constraint(&nodes[1]);
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::East);
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::East);
    graph
}

fn graph_one_node() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    add_node_to_layer(&graph, &layer);
    graph
}

fn graph_more_complex_in_layer() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);
    let left_nodes = add_nodes_to_layer(&graph, &layers[0], 4);
    let middle_nodes = add_nodes_to_layer(&graph, &layers[1], 3);
    let right_node = add_node_to_layer(&graph, &layers[2]);

    set_fixed_order_constraint(&middle_nodes[0]);
    set_fixed_order_constraint(&middle_nodes[1]);

    east_west_edge_from_to(&left_nodes[1], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[3], &middle_nodes[1]);
    east_west_edge_from_to(&left_nodes[2], &middle_nodes[1]);
    add_in_layer_edge(&middle_nodes[0], &middle_nodes[1], PortSide::West);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);
    add_in_layer_edge(&middle_nodes[0], &middle_nodes[2], PortSide::West);
    add_in_layer_edge(&middle_nodes[0], &middle_nodes[1], PortSide::East);
    east_west_edge_from_to(&middle_nodes[0], &right_node);
    graph
}

fn graph_in_layer_edges_fixed_port_order_in_layer_and_in_between_layer_crossing() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 2);
    let left_node = add_node_to_layer(&graph, &layers[0]);
    let right_nodes = add_nodes_to_layer(&graph, &layers[1], 3);

    set_fixed_order_constraint(&right_nodes[1]);
    east_west_edge_from_to(&left_node, &right_nodes[1]);
    add_in_layer_edge(&right_nodes[0], &right_nodes[1], PortSide::West);
    add_in_layer_edge(&right_nodes[1], &right_nodes[2], PortSide::West);
    graph
}

fn graph_one_layer_with_in_layer_crossings() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 4);
    add_in_layer_edge(&nodes[0], &nodes[2], PortSide::West);
    add_in_layer_edge(&nodes[1], &nodes[3], PortSide::West);
    graph
}

fn graph_in_layer_edges_multiple_edges_into_single_port() -> LGraphRef {
    let graph = new_graph();
    let layer_two = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &layer_two);
    let layer_one = make_layer(&graph);
    let right_nodes = add_nodes_to_layer(&graph, &layer_one, 4);

    add_in_layer_edge(&right_nodes[1], &right_nodes[3], PortSide::West);

    let left_port = add_port_on_side(&left_node, PortSide::East);
    let right_top_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let right_middle_port = add_port_on_side(&right_nodes[2], PortSide::West);
    add_edge_between_ports(&left_port, &right_middle_port);
    add_edge_between_ports(&right_top_port, &right_middle_port);
    graph
}

fn graph_in_layer_one_layer_no_crossings() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 4);
    add_in_layer_edge(&nodes[0], &nodes[3], PortSide::West);
    add_in_layer_edge(&nodes[1], &nodes[2], PortSide::West);
    graph
}

fn graph_in_layer_edges_fixed_port_order_in_layer_crossing() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 3);
    set_fixed_order_constraint(&nodes[1]);
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::West);
    add_in_layer_edge(&nodes[1], &nodes[2], PortSide::West);
    graph
}

fn graph_fixed_port_order_two_in_layer_edges_cross_each_other() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 3);
    set_fixed_order_constraint(&nodes[0]);
    add_in_layer_edge(&nodes[0], &nodes[2], PortSide::West);
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::West);
    graph
}

fn graph_multiple_in_between_layer_edges_into_node_with_no_fixed_port_order_cause_crossings(
) -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_layer = make_layer(&graph);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    add_in_layer_edge(&right_nodes[0], &right_nodes[2], PortSide::West);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    graph
}

#[test]
fn ignores_in_between_layer_edges() {
    let graph = graph_cross_formed();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn count_in_layer_edge_with_normal_edge_crossing() {
    let graph = graph_in_layer_edges();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn crossings_when_switched() {
    let graph = graph_in_layer_edges_crossings_when_switched();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 1);
}

#[test]
fn in_layer_edge_on_lower_node() {
    let graph = graph_in_layer_edges();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn switch_node_order() {
    let graph = graph_in_layer_edges();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.switch_order(1, 2);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn fixed_port_order_crossing_to_in_between_layer_edge() {
    let graph = graph_in_layer_edges_with_crossings_to_between_layer_edge_with_fixed_port_order();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 2);
    harness.switch_order(0, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 1);
}

#[test]
fn fixed_port_order_crossings_and_normal_edge_crossings() {
    let graph = graph_in_layer_edges_with_fixed_port_order_and_normal_edge_crossings();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 1);
    harness.switch_order(0, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 2);
}

#[test]
fn ignores_self_loops() {
    let graph = graph_cross_with_many_self_loops();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn crossings_on_both_sides() {
    let graph = graph_in_layer_crossings_on_both_sides();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn fixed_port_order_in_layer_no_crossings() {
    let graph = graph_fixed_port_order_in_layer_edges_dont_cross_each_other();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn fixed_port_order_in_layer_with_always_remaining_crossings_not_counted() {
    let graph = graph_fixed_port_order_in_layer_edges_with_crossings();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 1);
}

#[test]
fn one_node() {
    let graph = graph_one_node();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 0);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn more_complex() {
    let graph = graph_more_complex_in_layer();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 6);
    assert_eq!(harness.lower_upper, 6);
}

#[test]
fn downward_in_layer_edges_on_lower_node() {
    let graph = graph_in_layer_edges_fixed_port_order_in_layer_and_in_between_layer_crossing();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 2);
}

#[test]
fn one_layer_in_layer_crossing_should_disappear_after_any_switch() {
    let graph = graph_one_layer_with_in_layer_crossings();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);
    harness.count_crossings(2, 3);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);

    harness.switch_order(0, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 1);

    harness.switch_order(0, 1);
    harness.switch_order(1, 2);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 1);

    harness.switch_order(1, 2);
    harness.switch_order(2, 3);
    harness.count_crossings(2, 3);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 1);
}

#[test]
fn more_than_one_edge_into_a_port() {
    let graph = graph_in_layer_edges_multiple_edges_into_single_port();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn in_between_layer_edges_into_node_with_no_fixed_port_order_cause_crossings() {
    let graph =
        graph_multiple_in_between_layer_edges_into_node_with_no_fixed_port_order_cause_crossings();
    let mut harness = InLayerCrossingHarness::new(&graph, 1);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 0);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 2);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn in_layer_edges_pass_each_other() {
    let graph = graph_in_layer_one_layer_no_crossings();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 1);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
    harness.count_crossings(2, 3);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 1);
}

#[test]
fn fixed_port_order_crossing_to_in_layer_edge() {
    let graph = graph_in_layer_edges_fixed_port_order_in_layer_crossing();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(1, 2);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn fixed_port_order_two_in_layer_edges_cross_each_other() {
    let graph = graph_fixed_port_order_two_in_layer_edges_cross_each_other();
    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 1);
    assert_eq!(harness.lower_upper, 0);
}

#[test]
fn multiple_edges_into_one_port_should_not_cause_crossing() {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let nodes = add_nodes_to_layer(&graph, &layer, 3);
    let port_side = PortSide::East;
    let port_one = add_port_on_side(&nodes[0], port_side);
    let port_two = add_port_on_side(&nodes[1], port_side);
    let port_three = add_port_on_side(&nodes[2], port_side);
    add_edge_between_ports(&port_one, &port_three);
    add_edge_between_ports(&port_two, &port_three);

    let mut harness = InLayerCrossingHarness::new(&graph, 0);
    harness.count_crossings(0, 1);
    assert_eq!(harness.upper_lower, 0);
    assert_eq!(harness.lower_upper, 0);
}
