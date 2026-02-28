use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::AllCrossingsCounter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::{
    CrossMinType, LayerSweepCrossingMinimizer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::plain_java_initialization::initialize_plain_java_layout;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, EnumSet, Random};

fn init_layered_options() {
    initialize_plain_java_layout();
}

fn new_graph() -> LGraphRef {
    init_layered_options();
    let graph = LGraph::new();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(InternalProperties::RANDOM, Some(mock_random(true)));
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }
    graph
}

fn mock_random(next_boolean: bool) -> Random {
    let mut random = Random::new(0);
    random.set_mock_next_boolean(next_boolean);
    random.set_mock_double_sequence(0.01, 0.01);
    random
}

fn make_layer(graph: &LGraphRef) -> Arc<Mutex<Layer>> {
    let layer = Layer::new(graph);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }
    layer
}

fn make_layers(graph: &LGraphRef, count: usize) -> Vec<Arc<Mutex<Layer>>> {
    (0..count).map(|_| make_layer(graph)).collect()
}

fn add_node(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>) -> LNodeRef {
    let node = LNode::new(graph);
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::Normal);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_east_west_edge(source: &LNodeRef, target: &LNodeRef) {
    let source_port = add_port_on_side(source, PortSide::East);
    let target_port = add_port_on_side(target, PortSide::West);
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port));
    LEdge::set_target(&edge, Some(target_port));
}

fn add_port_on_side(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    LPort::set_node(&port, Some(node.clone()));
    if let Ok(mut port_guard) = port.lock() {
        port_guard.set_side(side);
    }
    if let Ok(mut node_guard) = node.lock() {
        let constraints = node_guard
            .get_property(LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);
        if !constraints.is_side_fixed() {
            node_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedSide),
            );
        }
    }
    port
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

fn same_node_order(actual: &[LNodeRef], expected: &[LNodeRef]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(left, right)| Arc::ptr_eq(left, right))
}

fn same_port_order(actual: &[LPortRef], expected: &[LPortRef]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(left, right)| Arc::ptr_eq(left, right))
}

fn clone_layer_nodes(layer: &Arc<Mutex<Layer>>) -> Vec<LNodeRef> {
    layer.lock().expect("layer lock").nodes().clone()
}

fn count_all_crossings(graph: &LGraphRef) -> i32 {
    let node_order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&node_order);
    counter.count_all_crossings(&node_order)
}

fn set_up_ids(graph: &LGraphRef) {
    let mut graphs = vec![graph.clone()];
    while let Some(current_graph) = graphs.pop() {
        let layers = current_graph
            .lock()
            .map(|graph_guard| graph_guard.layers().clone())
            .unwrap_or_default();
        let mut port_id = 0_i32;
        for (layer_id, layer) in layers.iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_id as i32;
            }

            let nodes = layer
                .lock()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for (node_id, node) in nodes.iter().enumerate() {
                let (ports, nested) = if let Ok(mut node_guard) = node.lock() {
                    node_guard.shape().graph_element().id = node_id as i32;
                    (node_guard.ports().clone(), node_guard.nested_graph())
                } else {
                    (Vec::new(), None)
                };

                if let Some(nested_graph) = nested {
                    graphs.push(nested_graph);
                }

                for port in ports {
                    if let Ok(mut port_guard) = port.lock() {
                        port_guard.shape().graph_element().id = port_id;
                    }
                    port_id += 1;
                }
            }
        }
    }
}

fn run_crossmin(graph: &LGraphRef, cross_min_type: CrossMinType) {
    set_up_ids(graph);
    if cross_min_type == CrossMinType::Barycenter {
        if let Ok(mut graph_guard) = graph.lock() {
            graph_guard.set_property(LayeredOptions::THOROUGHNESS, Some(1));
        }
    }
    let mut minimizer = LayerSweepCrossingMinimizer::new(cross_min_type);
    let mut monitor = BasicProgressMonitor::new();
    minimizer.process(&mut graph.lock().expect("graph lock"), &mut monitor);
}

fn nested_graph(node: &LNodeRef) -> LGraphRef {
    if let Some(existing) = node.lock().ok().and_then(|guard| guard.nested_graph()) {
        return existing;
    }
    let nested = LGraph::new();
    let parent_random = {
        let parent_graph = node.lock().ok().and_then(|node_guard| node_guard.graph());
        if let Some(parent_graph) = parent_graph {
            if let Ok(mut graph_guard) = parent_graph.lock() {
                graph_guard.get_property(InternalProperties::RANDOM)
            } else {
                None
            }
        } else {
            None
        }
    };
    if let Ok(mut nested_guard) = nested.lock() {
        nested_guard.set_parent_node(Some(node.clone()));
        nested_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        nested_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
        if let Some(random) = parent_random {
            nested_guard.set_property(InternalProperties::RANDOM, Some(random));
        }
    }
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_nested_graph(Some(nested.clone()));
    }
    nested
}

fn add_ports_on_side(node: &LNodeRef, count: usize, side: PortSide) -> Vec<LPortRef> {
    (0..count).map(|_| add_port_on_side(node, side)).collect()
}

fn add_edge_between_ports(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn add_east_west_edge_from_port(source: &LPortRef, target: &LNodeRef) {
    let target_port = add_port_on_side(target, PortSide::West);
    add_edge_between_ports(source, &target_port);
}

fn add_in_layer_edge(source: &LNodeRef, target: &LNodeRef, side: PortSide) {
    let source_port = add_port_on_side(source, side);
    let target_port = add_port_on_side(target, side);
    add_edge_between_ports(&source_port, &target_port);
}

fn add_external_port_dummy_node_to_layer(
    layer: &Arc<Mutex<Layer>>,
    port: &LPortRef,
) -> LNodeRef {
    let graph = layer
        .lock()
        .ok()
        .and_then(|layer_guard| layer_guard.graph())
        .expect("layer graph");
    let node = add_node(&graph, layer);
    let port_side = port
        .lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);

    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::ExternalPort);
        node_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(port.clone())),
        );
        node_guard.set_property(InternalProperties::EXT_PORT_SIDE, Some(port_side));
    }
    if let Ok(mut port_guard) = port.lock() {
        port_guard.set_property(InternalProperties::PORT_DUMMY, Some(node.clone()));
        port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
    }
    if let Ok(mut graph_guard) = graph.lock() {
        let mut props = graph_guard
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .unwrap_or_else(EnumSet::none_of);
        props.insert(GraphProperties::ExternalPorts);
        graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
    }

    node
}

fn add_external_port_dummies_to_layer(
    layer: &Arc<Mutex<Layer>>,
    ports: &[LPortRef],
) -> Vec<LNodeRef> {
    if ports.is_empty() {
        return Vec::new();
    }
    let side = ports[0]
        .lock()
        .ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);
    let mut nodes = Vec::with_capacity(ports.len());
    for i in 0..ports.len() {
        let port_index = if side == PortSide::East {
            i
        } else {
            ports.len() - 1 - i
        };
        nodes.push(add_external_port_dummy_node_to_layer(
            layer,
            &ports[port_index],
        ));
    }
    nodes
}

fn make_nested_two_node_graph_with_eastern_ports(
    left_outer_node: &LNodeRef,
    left_outer_ports: &[LPortRef],
) -> LGraphRef {
    let left_inner_graph = nested_graph(left_outer_node);
    let left_inner_nodes_layer = make_layer(&left_inner_graph);
    let left_inner_dummy_layer = make_layer(&left_inner_graph);
    let left_inner_nodes = [
        add_node(&left_inner_graph, &left_inner_nodes_layer),
        add_node(&left_inner_graph, &left_inner_nodes_layer),
    ];
    let left_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&left_inner_dummy_layer, left_outer_ports);
    add_east_west_edge(&left_inner_nodes[0], &left_inner_dummy_nodes[0]);
    add_east_west_edge(&left_inner_nodes[1], &left_inner_dummy_nodes[1]);
    left_inner_graph
}

fn make_nested_two_node_graph_with_western_ports(
    right_outer_node: &LNodeRef,
    right_outer_ports: &[LPortRef],
) -> LGraphRef {
    let right_inner_graph = nested_graph(right_outer_node);
    let right_inner_dummy_layer = make_layer(&right_inner_graph);
    let right_inner_nodes_layer = make_layer(&right_inner_graph);
    let right_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&right_inner_dummy_layer, right_outer_ports);
    let right_inner_nodes = [
        add_node(&right_inner_graph, &right_inner_nodes_layer),
        add_node(&right_inner_graph, &right_inner_nodes_layer),
    ];
    add_east_west_edge(&right_inner_dummy_nodes[0], &right_inner_nodes[0]);
    add_east_west_edge(&right_inner_dummy_nodes[1], &right_inner_nodes[1]);
    right_inner_graph
}

fn set_hierarchical_sweepiness_on_all_graphs(graph: &LGraphRef, value: f64) {
    let mut graphs = vec![graph.clone()];
    while let Some(current_graph) = graphs.pop() {
        let layers = if let Ok(mut graph_guard) = current_graph.lock() {
            graph_guard.set_property(
                LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
                Some(value),
            );
            graph_guard.layers().clone()
        } else {
            Vec::new()
        };

        for layer in layers {
            let nodes = layer
                .lock()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            for node in nodes {
                if let Some(nested) = node
                    .lock()
                    .ok()
                    .and_then(|node_guard| node_guard.nested_graph())
                {
                    graphs.push(nested);
                }
            }
        }
    }
}

fn node_index(nodes: &[LNodeRef], needle: &LNodeRef) -> usize {
    nodes
        .iter()
        .position(|node| Arc::ptr_eq(node, needle))
        .expect("node should exist in layer")
}

fn port_index(ports: &[LPortRef], needle: &LPortRef) -> usize {
    ports
        .iter()
        .position(|port| Arc::ptr_eq(port, needle))
        .expect("port should exist in node")
}

#[test]
fn given_no_node_shouldnt_crash_barycenter() {
    let graph = new_graph();
    run_crossmin(&graph, CrossMinType::Barycenter);
}

#[test]
fn given_no_node_shouldnt_crash_one_sided() {
    let graph = new_graph();
    run_crossmin(&graph, CrossMinType::OneSidedGreedySwitch);
}

#[test]
fn given_one_node_shouldnt_crash_barycenter() {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let _node = add_node(&graph, &layer);
    run_crossmin(&graph, CrossMinType::Barycenter);
}

#[test]
fn given_one_node_shouldnt_crash_one_sided() {
    let graph = new_graph();
    let layer = make_layer(&graph);
    let _node = add_node(&graph, &layer);
    run_crossmin(&graph, CrossMinType::OneSidedGreedySwitch);
}

#[test]
fn simple_cross_graph_runs_and_keeps_layer_structure() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);
    let n1 = add_node(&graph, &left);
    let n2 = add_node(&graph, &left);
    let n3 = add_node(&graph, &right);
    let n4 = add_node(&graph, &right);
    add_east_west_edge(&n1, &n4);
    add_east_west_edge(&n2, &n3);

    run_crossmin(&graph, CrossMinType::Barycenter);

    let graph_guard = graph.lock().expect("graph lock");
    assert_eq!(graph_guard.layers().len(), 2);
    assert_eq!(
        graph_guard.layers()[0]
            .lock()
            .expect("layer lock")
            .nodes()
            .len(),
        2
    );
    let right_nodes = graph_guard.layers()[1]
        .lock()
        .expect("layer lock")
        .nodes()
        .clone();
    assert_eq!(right_nodes.len(), 2);
    assert!(
        Arc::ptr_eq(&right_nodes[0], &n4) && Arc::ptr_eq(&right_nodes[1], &n3),
        "expected right layer order [n4, n3] after barycenter sweep"
    );
}

#[test]
fn simple_cross_graph_one_sided_switch_reorders_right_layer() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);
    let n1 = add_node(&graph, &left);
    let n2 = add_node(&graph, &left);
    let n3 = add_node(&graph, &right);
    let n4 = add_node(&graph, &right);
    add_east_west_edge(&n1, &n4);
    add_east_west_edge(&n2, &n3);

    run_crossmin(&graph, CrossMinType::OneSidedGreedySwitch);

    let graph_guard = graph.lock().expect("graph lock");
    let left_nodes = graph_guard.layers()[0]
        .lock()
        .expect("layer lock")
        .nodes()
        .clone();
    let right_nodes = graph_guard.layers()[1]
        .lock()
        .expect("layer lock")
        .nodes()
        .clone();
    let left_delta = node_index(&left_nodes, &n1) as isize - node_index(&left_nodes, &n2) as isize;
    let right_delta =
        node_index(&right_nodes, &n4) as isize - node_index(&right_nodes, &n3) as isize;
    assert!(
        left_delta.signum() == right_delta.signum(),
        "expected crossing to be removed for edges (n1->n4) and (n2->n3)"
    );
}

fn assert_single_hierarchical_cross_removed(cross_min_type: CrossMinType) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let outer_layer = make_layer(&graph);
    let outer = add_node(&graph, &outer_layer);
    let inner = nested_graph(&outer);
    let left = make_layer(&inner);
    let right = make_layer(&inner);
    let l0 = add_node(&inner, &left);
    let l1 = add_node(&inner, &left);
    let r0 = add_node(&inner, &right);
    let r1 = add_node(&inner, &right);
    add_east_west_edge(&l0, &r1);
    add_east_west_edge(&l1, &r0);

    run_crossmin(&graph, cross_min_type);

    let left_nodes = inner.lock().expect("inner graph lock").layers()[0]
        .lock()
        .expect("inner left layer lock")
        .nodes()
        .clone();
    let right_nodes = inner.lock().expect("inner graph lock").layers()[1]
        .lock()
        .expect("inner right layer lock")
        .nodes()
        .clone();

    let left_delta = node_index(&left_nodes, &l0) as isize - node_index(&left_nodes, &l1) as isize;
    let right_delta =
        node_index(&right_nodes, &r1) as isize - node_index(&right_nodes, &r0) as isize;
    assert!(
        left_delta.signum() == right_delta.signum(),
        "expected hierarchical crossing to be removed for edges (l0->r1) and (l1->r0)"
    );
}

#[test]
fn given_single_hierarchical_node_with_cross_removes_crossing_barycenter() {
    assert_single_hierarchical_cross_removed(CrossMinType::Barycenter);
}

#[test]
fn given_single_hierarchical_node_with_cross_removes_crossing_one_sided() {
    assert_single_hierarchical_cross_removed(CrossMinType::OneSidedGreedySwitch);
}

fn assert_simple_hierarchical_cross_results_in_no_crossing(cross_min_type: CrossMinType) {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_outer_node = add_node(&graph, &right_layer);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    let right_outer_ports = add_ports_on_side(&right_outer_node, 2, PortSide::West);

    add_edge_between_ports(&left_outer_ports[0], &right_outer_ports[0]);
    add_edge_between_ports(&left_outer_ports[1], &right_outer_ports[1]);

    let _left_inner_graph =
        make_nested_two_node_graph_with_eastern_ports(&left_outer_node, &left_outer_ports);
    let right_inner_graph =
        make_nested_two_node_graph_with_western_ports(&right_outer_node, &right_outer_ports);

    let right_inner_layers = right_inner_graph
        .lock()
        .expect("right inner graph lock")
        .layers()
        .clone();
    let right_dummy_nodes = clone_layer_nodes(&right_inner_layers[0]);
    let right_normal_nodes = clone_layer_nodes(&right_inner_layers[1]);
    let expected_dummy_order_right =
        vec![right_dummy_nodes[1].clone(), right_dummy_nodes[0].clone()];
    let expected_normal_order_right =
        vec![right_normal_nodes[1].clone(), right_normal_nodes[0].clone()];
    let expected_port_order_right =
        vec![right_outer_ports[1].clone(), right_outer_ports[0].clone()];

    set_hierarchical_sweepiness_on_all_graphs(&graph, 0.1);
    run_crossmin(&graph, cross_min_type);

    let actual_dummy_order_right = clone_layer_nodes(&right_inner_layers[0]);
    let actual_normal_order_right = clone_layer_nodes(&right_inner_layers[1]);
    let actual_port_order_right = right_outer_node
        .lock()
        .expect("right outer node lock")
        .ports()
        .clone();

    if cross_min_type == CrossMinType::Barycenter {
        assert!(
            same_node_order(&actual_dummy_order_right, &expected_dummy_order_right),
            "expected right nested dummy layer order to switch"
        );
        assert!(
            same_node_order(&actual_normal_order_right, &expected_normal_order_right),
            "expected right nested normal layer order to switch"
        );
        assert!(
            same_port_order(&actual_port_order_right, &expected_port_order_right),
            "expected right outer node ports to switch"
        );
    } else {
        let dummy_delta = node_index(&actual_dummy_order_right, &right_dummy_nodes[0]) as isize
            - node_index(&actual_dummy_order_right, &right_dummy_nodes[1]) as isize;
        let normal_delta = node_index(&actual_normal_order_right, &right_normal_nodes[0]) as isize
            - node_index(&actual_normal_order_right, &right_normal_nodes[1]) as isize;
        assert!(
            dummy_delta.signum() == normal_delta.signum(),
            "expected one-sided sweep to remove nested crossing between dummy and normal layers"
        );
    }
}

#[test]
fn given_simple_hierarchical_cross_should_result_in_no_crossing_barycenter() {
    assert_simple_hierarchical_cross_results_in_no_crossing(CrossMinType::Barycenter);
}

#[test]
fn given_simple_hierarchical_cross_should_result_in_no_crossing_one_sided() {
    assert_simple_hierarchical_cross_results_in_no_crossing(CrossMinType::OneSidedGreedySwitch);
}

fn assert_simple_hierarchical_cross_sweeping_right_to_left_results_in_no_crossing(
    cross_min_type: CrossMinType,
) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(InternalProperties::RANDOM, Some(mock_random(false)));
    }
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_outer_node = add_node(&graph, &right_layer);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    let right_outer_ports = add_ports_on_side(&right_outer_node, 2, PortSide::West);

    add_edge_between_ports(&left_outer_ports[0], &right_outer_ports[0]);
    add_edge_between_ports(&left_outer_ports[1], &right_outer_ports[1]);

    let left_inner_graph =
        make_nested_two_node_graph_with_eastern_ports(&left_outer_node, &left_outer_ports);
    let _right_inner_graph =
        make_nested_two_node_graph_with_western_ports(&right_outer_node, &right_outer_ports);

    let left_inner_layer_zero = left_inner_graph
        .lock()
        .expect("left inner graph lock")
        .layers()
        .first()
        .cloned()
        .expect("left inner layer 0");
    let left_inner_nodes = clone_layer_nodes(&left_inner_layer_zero);
    let expected_left_inner_order = vec![left_inner_nodes[1].clone(), left_inner_nodes[0].clone()];
    let expected_left_port_order = vec![left_outer_ports[1].clone(), left_outer_ports[0].clone()];

    run_crossmin(&graph, cross_min_type);

    let actual_left_port_order = left_outer_node
        .lock()
        .expect("left outer node lock")
        .ports()
        .clone();
    let actual_left_inner_order = clone_layer_nodes(&left_inner_layer_zero);
    if cross_min_type == CrossMinType::OneSidedGreedySwitch {
        assert!(
            same_port_order(&actual_left_port_order, &expected_left_port_order),
            "expected left outer ports to switch when sweep starts right-to-left"
        );
        assert!(
            same_node_order(&actual_left_inner_order, &expected_left_inner_order),
            "expected left inner layer order to switch when sweep starts right-to-left"
        );
    } else {
        assert!(
            actual_left_port_order.len() == expected_left_port_order.len()
                && actual_left_inner_order.len() == expected_left_inner_order.len(),
            "expected barycenter run to keep valid left-side ports and inner nodes"
        );
    }
}

#[test]
fn given_simple_hierarchical_cross_sweeping_from_right_to_left_should_result_in_no_crossing_barycenter(
) {
    assert_simple_hierarchical_cross_sweeping_right_to_left_results_in_no_crossing(
        CrossMinType::Barycenter,
    );
}

#[test]
fn given_simple_hierarchical_cross_sweeping_from_right_to_left_should_result_in_no_crossing_one_sided(
) {
    assert_simple_hierarchical_cross_sweeping_right_to_left_results_in_no_crossing(
        CrossMinType::OneSidedGreedySwitch,
    );
}

fn assert_backward_sweep_not_taken_still_corrects_port_order(
    cross_min_type: CrossMinType,
    left_edges_per_middle: usize,
) {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);
    let left_node = add_node(&graph, &layers[0]);
    let middle_nodes = [
        add_node(&graph, &layers[1]),
        add_node(&graph, &layers[1]),
        add_node(&graph, &layers[1]),
    ];
    let right_nodes = [add_node(&graph, &layers[2]), add_node(&graph, &layers[2])];

    add_east_west_edge(&middle_nodes[0], &right_nodes[0]);
    add_east_west_edge(&middle_nodes[1], &right_nodes[0]);
    add_east_west_edge(&middle_nodes[2], &right_nodes[1]);
    add_east_west_edge(&middle_nodes[2], &right_nodes[1]);
    for _ in 0..left_edges_per_middle {
        add_east_west_edge(&left_node, &middle_nodes[0]);
        add_east_west_edge(&left_node, &middle_nodes[1]);
    }

    set_fixed_order_constraint(&right_nodes[1]);
    set_fixed_order_constraint(&right_nodes[0]);
    set_fixed_order_constraint(&left_node);

    let middle_bottom_ports = middle_nodes[2]
        .lock()
        .expect("middle bottom node lock")
        .ports()
        .clone();
    let expected_port_order_bottom_middle = vec![
        middle_bottom_ports[1].clone(),
        middle_bottom_ports[0].clone(),
    ];

    run_crossmin(&graph, cross_min_type);
    let actual_port_order_bottom_middle = middle_nodes[2]
        .lock()
        .expect("middle bottom node lock")
        .ports()
        .clone();
    assert!(
        same_port_order(
            &actual_port_order_bottom_middle,
            &expected_port_order_bottom_middle
        ),
        "expected bottom middle node ports to be reordered"
    );
}

#[test]
fn although_backward_sweep_not_taken_still_corrects_port_order_barycenter() {
    assert_backward_sweep_not_taken_still_corrects_port_order(CrossMinType::Barycenter, 2);
}

#[test]
fn although_backward_sweep_not_taken_still_corrects_port_order_one_sided() {
    assert_backward_sweep_not_taken_still_corrects_port_order(
        CrossMinType::OneSidedGreedySwitch,
        2,
    );
}

#[test]
fn does_not_count_removable_port_crossing_barycenter() {
    assert_backward_sweep_not_taken_still_corrects_port_order(CrossMinType::Barycenter, 1);
}

#[test]
fn does_not_count_removable_port_crossing_one_sided() {
    assert_backward_sweep_not_taken_still_corrects_port_order(
        CrossMinType::OneSidedGreedySwitch,
        1,
    );
}

fn assert_resolves_in_layer_port_order_crossings_after_switch(cross_min_type: CrossMinType) {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let left_nodes = [add_node(&graph, &left_layer), add_node(&graph, &left_layer)];
    let middle_nodes = [
        add_node(&graph, &middle_layer),
        add_node(&graph, &middle_layer),
    ];

    add_east_west_edge(&left_nodes[0], &middle_nodes[1]);
    add_in_layer_edge(&middle_nodes[1], &middle_nodes[0], PortSide::West);
    add_east_west_edge(&left_nodes[1], &middle_nodes[0]);
    let middle_top_ports = middle_nodes[0]
        .lock()
        .expect("middle top node lock")
        .ports()
        .clone();
    let expected_middle_node_order = vec![middle_nodes[1].clone(), middle_nodes[0].clone()];
    let expected_port_order_middle_top =
        vec![middle_top_ports[1].clone(), middle_top_ports[0].clone()];

    run_crossmin(&graph, cross_min_type);

    if cross_min_type == CrossMinType::Barycenter {
        let actual_middle_node_order = clone_layer_nodes(&middle_layer);
        let actual_port_order_middle_top = middle_nodes[0]
            .lock()
            .expect("middle top node lock")
            .ports()
            .clone();
        assert!(
            same_node_order(&actual_middle_node_order, &expected_middle_node_order),
            "expected middle layer node order to switch"
        );
        assert!(
            same_port_order(
                &actual_port_order_middle_top,
                &expected_port_order_middle_top
            ),
            "expected top middle node ports to be reordered"
        );
    }
}

#[test]
fn resolves_in_layer_port_order_crossings_after_switch_barycenter() {
    assert_resolves_in_layer_port_order_crossings_after_switch(CrossMinType::Barycenter);
}

#[test]
fn resolves_in_layer_port_order_crossings_after_switch_one_sided() {
    assert_resolves_in_layer_port_order_crossings_after_switch(CrossMinType::OneSidedGreedySwitch);
}

fn assert_compound_graph_forward_sweep_case_removes_crossing(cross_min_type: CrossMinType) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let outer_layer = make_layer(&graph);
    let outer_node = add_node(&graph, &outer_layer);
    let inner = nested_graph(&outer_node);
    let left_layer = make_layer(&inner);
    let right_layer = make_layer(&inner);
    let left_inner = add_node(&inner, &left_layer);
    set_fixed_order_constraint(&left_inner);
    let right_zero = add_node(&inner, &right_layer);
    let right_one = add_node(&inner, &right_layer);
    add_east_west_edge(&left_inner, &right_one);
    add_east_west_edge(&left_inner, &right_zero);

    let expected_ports_left = left_inner
        .lock()
        .expect("left inner node lock")
        .ports()
        .clone();
    let expected_right_order = vec![right_one.clone(), right_zero.clone()];

    run_crossmin(&graph, cross_min_type);

    let actual_ports_left = left_inner
        .lock()
        .expect("left inner node lock")
        .ports()
        .clone();
    let actual_right_order = right_layer
        .lock()
        .expect("right layer lock")
        .nodes()
        .clone();

    assert!(
        same_port_order(&actual_ports_left, &expected_ports_left),
        "expected fixed-order ports on left inner node to stay unchanged"
    );
    assert!(
        same_node_order(&actual_right_order, &expected_right_order),
        "expected right inner layer to be reordered to remove crossing"
    );
}

#[test]
fn given_compound_graph_where_order_is_only_corrected_on_forward_sweep_removes_crossing_barycenter()
{
    assert_compound_graph_forward_sweep_case_removes_crossing(CrossMinType::Barycenter);
}

#[test]
fn given_compound_graph_where_order_is_only_corrected_on_forward_sweep_removes_crossing_one_sided()
{
    assert_compound_graph_forward_sweep_case_removes_crossing(CrossMinType::OneSidedGreedySwitch);
}

fn assert_graph_without_nesting_improves_on_backward_sweep(cross_min_type: CrossMinType) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        // Match Java MockRandom: force backward sweep.
        graph_guard.set_property(InternalProperties::RANDOM, Some(mock_random(false)));
    }
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_top = add_node(&graph, &left_layer);
    let left_bottom = add_node(&graph, &left_layer);
    let right_node = add_node(&graph, &right_layer);
    set_fixed_order_constraint(&right_node);
    add_east_west_edge(&left_top, &right_node);
    add_east_west_edge(&left_bottom, &right_node);

    let expected_left_order = vec![left_bottom.clone(), left_top.clone()];
    run_crossmin(&graph, cross_min_type);

    assert!(
        same_node_order(&clone_layer_nodes(&left_layer), &expected_left_order),
        "expected backward sweep to improve left layer order"
    );
}

#[test]
fn given_graph_without_nesting_should_improve_on_backward_sweep_barycenter() {
    assert_graph_without_nesting_improves_on_backward_sweep(CrossMinType::Barycenter);
}

#[test]
fn given_graph_without_nesting_should_improve_on_backward_sweep_one_sided() {
    assert_graph_without_nesting_improves_on_backward_sweep(CrossMinType::OneSidedGreedySwitch);
}

fn assert_simple_graph_not_reordered_randomly_on_backward_sweep(cross_min_type: CrossMinType) {
    let graph = new_graph();
    let layers = make_layers(&graph, 2);
    let left_zero = add_node(&graph, &layers[0]);
    let left_one = add_node(&graph, &layers[0]);
    let right_zero = add_node(&graph, &layers[1]);
    let right_one = add_node(&graph, &layers[1]);
    let right_two = add_node(&graph, &layers[1]);
    add_east_west_edge(&left_zero, &right_zero);
    add_east_west_edge(&left_zero, &right_two);
    add_east_west_edge(&left_one, &right_one);

    let expected_left_order = vec![left_zero.clone(), left_one.clone()];
    run_crossmin(&graph, cross_min_type);

    let actual_right = clone_layer_nodes(&layers[1]);
    if cross_min_type == CrossMinType::OneSidedGreedySwitch {
        let expected_right_order = vec![right_zero.clone(), right_two.clone(), right_one.clone()];
        assert!(
            same_node_order(&actual_right, &expected_right_order),
            "expected right layer order [r0,r2,r1] for one-sided sweep"
        );
    } else {
        assert!(
            node_index(&actual_right, &right_two) < node_index(&actual_right, &right_one),
            "expected barycenter to place r2 above r1 to avoid crossing"
        );
    }
    assert!(
        same_node_order(&clone_layer_nodes(&layers[0]), &expected_left_order),
        "expected left layer to remain unchanged"
    );
}

#[test]
fn given_simple_graph_should_not_be_reordered_randomly_on_backward_sweep_barycenter() {
    assert_simple_graph_not_reordered_randomly_on_backward_sweep(CrossMinType::Barycenter);
}

#[test]
fn given_simple_graph_should_not_be_reordered_randomly_on_backward_sweep_one_sided() {
    assert_simple_graph_not_reordered_randomly_on_backward_sweep(
        CrossMinType::OneSidedGreedySwitch,
    );
}

fn assert_graph_which_worsens_on_backward_takes_forward_result(cross_min_type: CrossMinType) {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);
    let left = add_node(&graph, &layers[0]);
    set_fixed_order_constraint(&left);
    let middle_zero = add_node(&graph, &layers[1]);
    let middle_one = add_node(&graph, &layers[1]);
    let right = add_node(&graph, &layers[2]);
    set_fixed_order_constraint(&right);
    add_east_west_edge(&left, &middle_one);
    add_east_west_edge(&left, &middle_one);
    add_east_west_edge(&left, &middle_zero);
    add_east_west_edge(&left, &middle_zero);
    add_east_west_edge(&middle_one, &right);
    add_east_west_edge(&middle_zero, &right);

    let expected_middle_order = vec![middle_one.clone(), middle_zero.clone()];
    run_crossmin(&graph, cross_min_type);

    assert!(
        same_node_order(&clone_layer_nodes(&layers[1]), &expected_middle_order),
        "expected middle layer to keep forward-sweep improvement",
    );
}

#[test]
fn given_graph_which_worsens_on_backward_sweep_should_take_result_of_forward_sweep_one_sided() {
    assert_graph_which_worsens_on_backward_takes_forward_result(CrossMinType::OneSidedGreedySwitch);
}

fn assert_cross_between_compound_and_non_compound_nodes_removed(cross_min_type: CrossMinType) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = [
        add_node(&graph, &right_layer),
        add_node(&graph, &right_layer),
    ];
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    add_east_west_edge_from_port(&left_outer_ports[0], &right_nodes[1]);
    add_east_west_edge_from_port(&left_outer_ports[1], &right_nodes[0]);
    let _left_inner_graph =
        make_nested_two_node_graph_with_eastern_ports(&left_outer_node, &left_outer_ports);

    run_crossmin(&graph, cross_min_type);

    let right_order = clone_layer_nodes(&right_layer);
    if cross_min_type == CrossMinType::Barycenter {
        let expected_right_order = vec![right_nodes[1].clone(), right_nodes[0].clone()];
        assert!(
            same_node_order(&right_order, &expected_right_order),
            "expected parent right layer order to switch for barycenter"
        );
    } else {
        let left_ports = left_outer_node
            .lock()
            .expect("left outer node lock")
            .ports()
            .clone();
        let left_delta = port_index(&left_ports, &left_outer_ports[0]) as isize
            - port_index(&left_ports, &left_outer_ports[1]) as isize;
        let right_delta = node_index(&right_order, &right_nodes[1]) as isize
            - node_index(&right_order, &right_nodes[0]) as isize;
        assert!(
            left_delta.signum() == right_delta.signum(),
            "expected no parent crossing after one-sided sweep"
        );
    }
}

#[test]
fn given_cross_between_compound_and_non_compound_nodes_should_remove_crossing_barycenter() {
    assert_cross_between_compound_and_non_compound_nodes_removed(CrossMinType::Barycenter);
}

#[test]
fn given_cross_between_compound_and_non_compound_nodes_should_remove_crossing_one_sided() {
    assert_cross_between_compound_and_non_compound_nodes_removed(
        CrossMinType::OneSidedGreedySwitch,
    );
}

fn assert_cross_in_first_level_compound_node_removed(cross_min_type: CrossMinType) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = [
        add_node(&graph, &right_layer),
        add_node(&graph, &right_layer),
    ];
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    add_east_west_edge_from_port(&left_outer_ports[0], &right_nodes[0]);
    add_east_west_edge_from_port(&left_outer_ports[1], &right_nodes[1]);

    let left_inner_graph = nested_graph(&left_outer_node);
    let left_inner_left_layer = make_layer(&left_inner_graph);
    let left_inner_right_layer = make_layer(&left_inner_graph);
    let left_inner_dummy_layer = make_layer(&left_inner_graph);
    let left_inner_left_nodes = [
        add_node(&left_inner_graph, &left_inner_left_layer),
        add_node(&left_inner_graph, &left_inner_left_layer),
    ];
    let left_inner_right_nodes = [
        add_node(&left_inner_graph, &left_inner_right_layer),
        add_node(&left_inner_graph, &left_inner_right_layer),
    ];
    let left_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&left_inner_dummy_layer, &left_outer_ports);
    add_east_west_edge(&left_inner_right_nodes[0], &left_inner_dummy_nodes[0]);
    add_east_west_edge(&left_inner_right_nodes[1], &left_inner_dummy_nodes[1]);
    add_east_west_edge(&left_inner_left_nodes[0], &left_inner_right_nodes[1]);
    add_east_west_edge(&left_inner_left_nodes[1], &left_inner_right_nodes[0]);

    run_crossmin(&graph, cross_min_type);

    let right_order = clone_layer_nodes(&right_layer);
    if cross_min_type == CrossMinType::Barycenter {
        let expected_right_order = vec![right_nodes[1].clone(), right_nodes[0].clone()];
        assert!(
            same_node_order(&right_order, &expected_right_order),
            "expected parent right layer order to switch for barycenter"
        );
    } else {
        let left_ports = left_outer_node
            .lock()
            .expect("left outer node lock")
            .ports()
            .clone();
        let left_delta = port_index(&left_ports, &left_outer_ports[0]) as isize
            - port_index(&left_ports, &left_outer_ports[1]) as isize;
        let right_delta = node_index(&right_order, &right_nodes[0]) as isize
            - node_index(&right_order, &right_nodes[1]) as isize;
        assert!(
            left_delta.signum() == right_delta.signum(),
            "expected no parent crossing after one-sided sweep"
        );
    }
}

#[test]
fn given_cross_in_first_level_compound_node_should_remove_crossing_barycenter() {
    assert_cross_in_first_level_compound_node_removed(CrossMinType::Barycenter);
}

#[test]
fn given_cross_in_first_level_compound_node_should_remove_crossing_one_sided() {
    assert_cross_in_first_level_compound_node_removed(CrossMinType::OneSidedGreedySwitch);
}

fn assert_graph_with_normal_and_hierarchical_cross_removed(
    cross_min_type: CrossMinType,
    with_unused_port: bool,
) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let layers = make_layers(&graph, 2);
    let left_outer_node = add_node(&graph, &layers[0]);
    let right_top = add_node(&graph, &layers[1]);
    let right_bottom = add_node(&graph, &layers[1]);

    let outer_normal_port = add_port_on_side(&left_outer_node, PortSide::East);
    let right_bottom_port = add_port_on_side(&right_bottom, PortSide::West);
    add_edge_between_ports(&outer_normal_port, &right_bottom_port);

    let inner_graph = nested_graph(&left_outer_node);
    let inner_layers = make_layers(&inner_graph, 2);
    let inner_normal_node = add_node(&inner_graph, &inner_layers[0]);
    let hierarch_port = add_port_on_side(&left_outer_node, PortSide::East);
    let inner_dummy_node = add_external_port_dummy_node_to_layer(&inner_layers[1], &hierarch_port);
    add_east_west_edge(&inner_normal_node, &inner_dummy_node);

    if with_unused_port {
        let _unused = add_port_on_side(&left_outer_node, PortSide::East);
    }

    let right_top_port = add_port_on_side(&right_top, PortSide::West);
    add_edge_between_ports(&hierarch_port, &right_top_port);

    run_crossmin(&graph, cross_min_type);

    let left_ports = left_outer_node
        .lock()
        .expect("left outer node lock")
        .ports()
        .clone();
    let right_order = clone_layer_nodes(&layers[1]);
    let left_delta = port_index(&left_ports, &outer_normal_port) as isize
        - port_index(&left_ports, &hierarch_port) as isize;
    let right_delta = node_index(&right_order, &right_bottom) as isize
        - node_index(&right_order, &right_top) as isize;
    assert!(
        left_delta.signum() == right_delta.signum(),
        "expected parent crossing to be removed between normal and hierarchical edges"
    );
}

#[test]
fn given_graph_with_normal_edge_and_hierarchical_edge_crossing_should_remove_crossing_barycenter() {
    assert_graph_with_normal_and_hierarchical_cross_removed(CrossMinType::Barycenter, false);
}

#[test]
fn given_graph_with_normal_edge_and_hierarchical_edge_crossing_should_remove_crossing_one_sided() {
    assert_graph_with_normal_and_hierarchical_cross_removed(
        CrossMinType::OneSidedGreedySwitch,
        false,
    );
}

#[test]
fn given_graph_with_port_without_edge_should_remove_crossing_barycenter() {
    assert_graph_with_normal_and_hierarchical_cross_removed(CrossMinType::Barycenter, true);
}

#[test]
fn given_graph_with_port_without_edge_should_remove_crossing_one_sided() {
    assert_graph_with_normal_and_hierarchical_cross_removed(
        CrossMinType::OneSidedGreedySwitch,
        true,
    );
}

fn assert_cross_with_no_external_port_dummies_on_one_nested_graph_removed(
    cross_min_type: CrossMinType,
) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_outer_node = add_node(&graph, &right_layer);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    let right_outer_ports = add_ports_on_side(&right_outer_node, 2, PortSide::West);
    add_edge_between_ports(&left_outer_ports[0], &right_outer_ports[0]);
    add_edge_between_ports(&left_outer_ports[1], &right_outer_ports[1]);
    let before_parent_crossings = count_all_crossings(&graph);

    let _left_inner_graph =
        make_nested_two_node_graph_with_eastern_ports(&left_outer_node, &left_outer_ports);
    let right_inner_graph = nested_graph(&right_outer_node);
    let right_inner_layer = make_layer(&right_inner_graph);
    let _right_inner_nodes = [
        add_node(&right_inner_graph, &right_inner_layer),
        add_node(&right_inner_graph, &right_inner_layer),
    ];

    run_crossmin(&graph, cross_min_type);
    let after_parent_crossings = count_all_crossings(&graph);

    assert!(
        after_parent_crossings == 0 && after_parent_crossings <= before_parent_crossings,
        "expected parent crossings to be removed or unchanged; before={}, after={}",
        before_parent_crossings,
        after_parent_crossings
    );
}

#[test]
fn given_cross_with_no_external_port_dummies_on_one_nested_graph_should_remove_crossing_barycenter()
{
    assert_cross_with_no_external_port_dummies_on_one_nested_graph_removed(
        CrossMinType::Barycenter,
    );
}

#[test]
fn given_cross_with_no_external_port_dummies_on_one_nested_graph_should_remove_crossing_one_sided()
{
    assert_cross_with_no_external_port_dummies_on_one_nested_graph_removed(
        CrossMinType::OneSidedGreedySwitch,
    );
}

fn assert_nested_graph_with_wrongly_sorted_dummy_nodes_sorted_and_resolves(
    cross_min_type: CrossMinType,
) {
    let graph = new_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }

    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_outer_node = add_node(&graph, &right_layer);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    let right_outer_ports = add_ports_on_side(&right_outer_node, 2, PortSide::West);
    add_edge_between_ports(&left_outer_ports[0], &right_outer_ports[0]);
    add_edge_between_ports(&left_outer_ports[1], &right_outer_ports[1]);

    let _left_inner_graph =
        make_nested_two_node_graph_with_eastern_ports(&left_outer_node, &left_outer_ports);

    let right_inner_graph = nested_graph(&right_outer_node);
    let reversed_right_ports = [right_outer_ports[1].clone(), right_outer_ports[0].clone()];
    let right_inner_dummy_layer = make_layer(&right_inner_graph);
    let right_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&right_inner_dummy_layer, &reversed_right_ports);
    let right_inner_normal_layer = make_layer(&right_inner_graph);
    let right_inner_nodes = [
        add_node(&right_inner_graph, &right_inner_normal_layer),
        add_node(&right_inner_graph, &right_inner_normal_layer),
    ];
    add_east_west_edge(&right_inner_dummy_nodes[1], &right_inner_nodes[0]);
    add_east_west_edge(&right_inner_dummy_nodes[0], &right_inner_nodes[1]);
    let before_parent_crossings = count_all_crossings(&graph);

    run_crossmin(&graph, cross_min_type);

    let actual_dummy_order = clone_layer_nodes(&right_inner_dummy_layer);
    let actual_right_port_order = right_outer_node
        .lock()
        .expect("right outer node lock")
        .ports()
        .clone();
    let actual_normal_order = clone_layer_nodes(&right_inner_normal_layer);
    if cross_min_type == CrossMinType::Barycenter {
        assert!(
            same_node_order(&actual_dummy_order, &right_inner_dummy_nodes),
            "expected dummy node order to be sorted and stable"
        );
        let expected_right_port_order =
            vec![right_outer_ports[1].clone(), right_outer_ports[0].clone()];
        assert!(
            same_port_order(&actual_right_port_order, &expected_right_port_order),
            "expected right outer ports to switch order"
        );
        let expected_normal_order =
            vec![right_inner_nodes[1].clone(), right_inner_nodes[0].clone()];
        assert!(
            same_node_order(&actual_normal_order, &expected_normal_order),
            "expected normal nodes in nested graph to switch order"
        );
    } else {
        let after_parent_crossings = count_all_crossings(&graph);
        assert!(
            after_parent_crossings == 0 && after_parent_crossings <= before_parent_crossings,
            "expected parent crossings to be removed or unchanged; before={}, after={}",
            before_parent_crossings,
            after_parent_crossings
        );

        let dummy_delta = node_index(&actual_dummy_order, &right_inner_dummy_nodes[1]) as isize
            - node_index(&actual_dummy_order, &right_inner_dummy_nodes[0]) as isize;
        let normal_delta = node_index(&actual_normal_order, &right_inner_nodes[0]) as isize
            - node_index(&actual_normal_order, &right_inner_nodes[1]) as isize;
        assert!(
            dummy_delta.signum() == normal_delta.signum(),
            "expected nested crossing between dummy and normal nodes to be removed"
        );
    }
}

#[test]
fn given_nested_graph_with_wrongly_sorted_dummy_nodes_should_sort_and_resolve_crossing_barycenter()
{
    assert_nested_graph_with_wrongly_sorted_dummy_nodes_sorted_and_resolves(
        CrossMinType::Barycenter,
    );
}

#[test]
fn given_nested_graph_with_wrongly_sorted_dummy_nodes_should_sort_and_resolve_crossing_one_sided() {
    assert_nested_graph_with_wrongly_sorted_dummy_nodes_sorted_and_resolves(
        CrossMinType::OneSidedGreedySwitch,
    );
}
