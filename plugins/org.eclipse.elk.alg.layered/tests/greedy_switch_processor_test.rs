use std::sync::OnceLock;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredMetaDataProvider, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::{
    CrossMinType, LayerSweepCrossingMinimizer,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, EnumSet, Random};

fn init_reflect() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
    });
}

fn new_graph() -> LGraphRef {
    init_reflect();
    let graph = LGraph::new();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
        graph_guard.set_property(InternalProperties::RANDOM, Some(mock_random(true)));
    }
    graph
}

fn mock_random(next_boolean: bool) -> Random {
    let mut random = Random::new(0);
    random.set_mock_next_boolean(next_boolean);
    random.set_mock_double_sequence(0.01, 0.01);
    random
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
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::Normal);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_nodes_to_layer(graph: &LGraphRef, layer: &LayerRef, count: usize) -> Vec<LNodeRef> {
    (0..count).map(|_| add_node_to_layer(graph, layer)).collect()
}

fn add_port_on_side(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    if let Ok(mut port_guard) = port.lock() {
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    if let Ok(mut node_guard) = node.lock() {
        let constraints = node_guard
            .get_property(LayeredOptions::PORT_CONSTRAINTS)
            .unwrap_or(PortConstraints::Undefined);
        if !constraints.is_side_fixed() {
            node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedSide));
        }
    }
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

fn set_in_layer_order_constraint(this_node: &LNodeRef, before_node: &LNodeRef) {
    if let Ok(mut node_guard) = this_node.lock() {
        node_guard.set_property(
            InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
            Some(vec![before_node.clone()]),
        );
    }
}

fn add_north_south_edge(
    side: PortSide,
    node_with_ns_ports: &LNodeRef,
    north_south_dummy: &LNodeRef,
    node_with_east_west_ports: &LNodeRef,
    node_with_east_west_ports_is_origin: bool,
) {
    let ns_layer_index = node_with_ns_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| layer.lock().ok().and_then(|layer_guard| layer_guard.index()))
        .unwrap_or(0);
    let other_layer_index = node_with_east_west_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| layer.lock().ok().and_then(|layer_guard| layer_guard.index()))
        .unwrap_or(0);
    let normal_node_east_of_ns = other_layer_index < ns_layer_index;
    let direction = if normal_node_east_of_ns {
        PortSide::West
    } else {
        PortSide::East
    };

    let target_node_port_side = direction.opposed();
    let normal_node_port = add_port_on_side(node_with_east_west_ports, target_node_port_side);
    let dummy_node_port = add_port_on_side(north_south_dummy, direction);

    if node_with_east_west_ports_is_origin {
        add_edge_between_ports(&normal_node_port, &dummy_node_port);
    } else {
        add_edge_between_ports(&dummy_node_port, &normal_node_port);
    }

    if let Ok(mut dummy_guard) = north_south_dummy.lock() {
        dummy_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node_with_ns_ports.clone()));
        dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LNode(node_with_ns_ports.clone())));
        dummy_guard.set_node_type(NodeType::NorthSouthPort);
    }

    let origin_port = add_port_on_side(node_with_ns_ports, side);
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(origin_port.clone())));
    }
    if let Ok(mut origin_port_guard) = origin_port.lock() {
        origin_port_guard.set_property(InternalProperties::PORT_DUMMY, Some(north_south_dummy.clone()));
    }

    if let Ok(mut node_guard) = node_with_ns_ports.lock() {
        let mut existing = node_guard
            .get_property(InternalProperties::BARYCENTER_ASSOCIATES)
            .unwrap_or_default();
        if existing.is_empty() {
            node_guard.set_property(
                InternalProperties::BARYCENTER_ASSOCIATES,
                Some(vec![north_south_dummy.clone()]),
            );
        } else {
            existing.push(north_south_dummy.clone());
            node_guard.set_property(InternalProperties::BARYCENTER_ASSOCIATES, Some(existing));
        }
    }

    if side == PortSide::North {
        if let Ok(mut dummy_guard) = north_south_dummy.lock() {
            let mut constraints = dummy_guard
                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                .unwrap_or_default();
            constraints.push(node_with_ns_ports.clone());
            dummy_guard.set_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS, Some(constraints));
        }
    } else if let Ok(mut node_guard) = node_with_ns_ports.lock() {
        let mut constraints = node_guard
            .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
            .unwrap_or_default();
        constraints.push(north_south_dummy.clone());
        node_guard.set_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS, Some(constraints));
    }

    if let Some(graph) = node_with_ns_ports.lock().ok().and_then(|node_guard| node_guard.graph()) {
        if let Ok(mut graph_guard) = graph.lock() {
            let mut props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            props.insert(GraphProperties::NorthSouthPorts);
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
        }
    }
}

fn set_up_ids(root: &LGraphRef) {
    let mut stack = vec![root.clone()];
    while let Some(graph) = stack.pop() {
        if let Ok(graph_guard) = graph.lock() {
            let layers = graph_guard.layers().clone();
            drop(graph_guard);
            let mut port_id = 0i32;
            for (layer_idx, layer) in layers.iter().enumerate() {
                if let Ok(mut layer_guard) = layer.lock() {
                    layer_guard.graph_element().id = layer_idx as i32;
                    for (node_idx, node) in layer_guard.nodes().iter().enumerate() {
                        if let Ok(mut node_guard) = node.lock() {
                            node_guard.shape().graph_element().id = node_idx as i32;
                            if let Some(nested) = node_guard.nested_graph() {
                                stack.push(nested);
                            }
                            for port in node_guard.ports_mut() {
                                if let Ok(mut port_guard) = port.lock() {
                                    port_guard.shape().graph_element().id = port_id;
                                }
                                port_id += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn nodes_in_layer(graph: &LGraphRef, layer_idx: usize) -> Vec<LNodeRef> {
    let graph_guard = graph.lock().expect("graph lock");
    let layer = graph_guard.layers().get(layer_idx).cloned().expect("layer");
    let nodes = layer.lock().expect("layer lock").nodes().clone();
    nodes
}

fn copy_of_nodes_in_layer(graph: &LGraphRef, layer_idx: usize) -> Vec<LNodeRef> {
    nodes_in_layer(graph, layer_idx)
}

fn copy_of_switch_order_of_nodes_in_layer(
    graph: &LGraphRef,
    upper: usize,
    lower: usize,
    layer_idx: usize,
) -> Vec<LNodeRef> {
    let mut nodes = copy_of_nodes_in_layer(graph, layer_idx);
    nodes.swap(upper, lower);
    nodes
}

fn get_copy_with_switched_order(upper: usize, lower: usize, nodes: &[LNodeRef]) -> Vec<LNodeRef> {
    let mut out = nodes.to_vec();
    out.swap(upper, lower);
    out
}

fn assert_layer_order(graph: &LGraphRef, layer_idx: usize, expected: &[LNodeRef], label: &str) {
    let actual = nodes_in_layer(graph, layer_idx);
    assert_eq!(actual.len(), expected.len(), "{label}: length");
    for (idx, (actual_node, expected_node)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            std::sync::Arc::ptr_eq(actual_node, expected_node),
            "{label}: index {idx}"
        );
    }
}

fn run_greedy_switcher(graph: &LGraphRef, greedy_type: CrossMinType) {
    let mut minimizer = LayerSweepCrossingMinimizer::new(greedy_type);
    let mut monitor = BasicProgressMonitor::new();
    let mut graph_guard = graph.lock().expect("graph lock");
    minimizer.process(&mut graph_guard, &mut monitor);
}

fn for_each_greedy_type<F: FnMut(CrossMinType)>(mut f: F) {
    let filter = std::env::var("ELK_GREEDY_TYPE")
        .ok()
        .map(|value| value.to_ascii_lowercase());
    for greedy_type in [
        CrossMinType::OneSidedGreedySwitch,
        CrossMinType::TwoSidedGreedySwitch,
    ] {
        if let Some(ref filter) = filter {
            let want_one = filter.starts_with("one");
            let want_two = filter.starts_with("two");
            if want_one && greedy_type != CrossMinType::OneSidedGreedySwitch {
                continue;
            }
            if want_two && greedy_type != CrossMinType::TwoSidedGreedySwitch {
                continue;
            }
        }
        f(greedy_type);
    }
}

fn graph_cross_formed() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let top_left = add_node_to_layer(&graph, &left_layer);
    let bottom_left = add_node_to_layer(&graph, &left_layer);
    let top_right = add_node_to_layer(&graph, &right_layer);
    let bottom_right = add_node_to_layer(&graph, &right_layer);

    east_west_edge_from_to(&top_left, &bottom_right);
    east_west_edge_from_to(&bottom_left, &top_right);
    set_up_ids(&graph);
    graph
}

fn graph_cross_formed_with_constraints_in_second_layer() -> LGraphRef {
    let graph = graph_cross_formed();
    let layer = graph
        .lock()
        .expect("graph lock")
        .layers()
        .get(1)
        .cloned()
        .expect("layer 1");
    let nodes = layer.lock().expect("layer lock").nodes().clone();
    set_in_layer_order_constraint(&nodes[0], &nodes[1]);
    set_up_ids(&graph);
    graph
}

fn graph_cross_formed_constraints_prevent_any_switch() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let top_left = add_node_to_layer(&graph, &left_layer);
    let bottom_left = add_node_to_layer(&graph, &left_layer);
    let top_right = add_node_to_layer(&graph, &right_layer);
    let bottom_right = add_node_to_layer(&graph, &right_layer);

    east_west_edge_from_to(&top_left, &bottom_right);
    east_west_edge_from_to(&bottom_left, &top_right);
    set_in_layer_order_constraint(&top_right, &bottom_right);
    set_in_layer_order_constraint(&top_left, &bottom_left);
    set_up_ids(&graph);
    graph
}

fn graph_one_node() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    add_node_to_layer(&graph, &layer);
    set_up_ids(&graph);
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
    set_up_ids(&graph);
    graph
}

fn graph_multiple_edges_between_same_nodes() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let top_left = add_node_to_layer(&graph, &left_layer);
    let bottom_left = add_node_to_layer(&graph, &left_layer);
    let top_right = add_node_to_layer(&graph, &right_layer);
    let bottom_right = add_node_to_layer(&graph, &right_layer);

    east_west_edge_from_to(&top_left, &bottom_right);
    east_west_edge_from_to(&top_left, &bottom_right);
    east_west_edge_from_to(&bottom_left, &top_right);
    east_west_edge_from_to(&bottom_left, &top_right);
    set_up_ids(&graph);
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
    let right_upper_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let right_middle_port = add_port_on_side(&right_nodes[1], PortSide::West);
    set_up_ids(&graph);

    add_edge_between_ports(&middle_upper_east, &right_upper_port);
    add_edge_between_ports(&middle_upper_east, &right_middle_port);
    add_edge_between_ports(&middle_upper_east, &right_middle_port);
    east_west_edge_from_port(&middle_lower_east, &right_nodes[2]);
    east_west_edge_from_port(&left_middle_port, &middle_nodes[0]);
    east_west_edge_from_node(&middle_nodes[1], &right_upper_port);
    east_west_edge_from_port(&left_middle_port, &middle_nodes[1]);
    east_west_edge_from_to(&left_nodes[2], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);

    set_up_ids(&graph);
    graph
}

fn graph_switch_only_one_sided() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);

    let left_nodes = add_nodes_to_layer(&graph, &layers[0], 2);
    let middle_nodes = add_nodes_to_layer(&graph, &layers[1], 2);
    let right_nodes = add_nodes_to_layer(&graph, &layers[2], 2);

    east_west_edge_from_to(&middle_nodes[0], &right_nodes[0]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &middle_nodes[0]);

    set_up_ids(&graph);
    graph
}

fn graph_which_could_be_worsened_by_switch() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);

    let left_nodes = add_nodes_to_layer(&graph, &layers[0], 2);
    let middle_nodes = add_nodes_to_layer(&graph, &layers[1], 2);
    let right_nodes = add_nodes_to_layer(&graph, &layers[2], 2);

    set_in_layer_order_constraint(&left_nodes[0], &left_nodes[1]);
    set_in_layer_order_constraint(&right_nodes[0], &right_nodes[1]);

    east_west_edge_from_to(&middle_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[1], &middle_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &middle_nodes[1]);

    set_up_ids(&graph);
    graph
}

fn graph_nodes_in_different_layout_units_prevent_switch() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 2);

    let left_nodes = add_nodes_to_layer(&graph, &layers[0], 2);
    let right_nodes = add_nodes_to_layer(&graph, &layers[1], 3);

    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    add_north_south_edge(
        PortSide::East,
        &right_nodes[2],
        &right_nodes[1],
        &left_nodes[0],
        true,
    );

    if let Ok(mut node_guard) = right_nodes[1].lock() {
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(right_nodes[2].clone()));
    }
    if let Ok(mut node_guard) = right_nodes[2].lock() {
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(right_nodes[2].clone()));
    }

    set_up_ids(&graph);
    graph
}

fn graph_north_south_downward_crossing() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    add_north_south_edge(
        PortSide::South,
        &left_nodes[0],
        &left_nodes[2],
        &right_nodes[1],
        false,
    );
    add_north_south_edge(
        PortSide::South,
        &left_nodes[0],
        &left_nodes[1],
        &right_nodes[0],
        false,
    );

    set_fixed_order_constraint(&left_nodes[0]);
    set_up_ids(&graph);
    graph
}

#[test]
fn should_switch_cross() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_formed();

        let expected_layer_one;
        let expected_layer_two;
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            expected_layer_one = copy_of_nodes_in_layer(&graph, 0);
            expected_layer_two = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 1);
        } else {
            expected_layer_one = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 0);
            expected_layer_two = copy_of_nodes_in_layer(&graph, 1);
        }

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 0, &expected_layer_one, "layer one");
        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
    });
}

#[test]
fn constraints_prevent_switch_in_second_layer() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_formed_with_constraints_in_second_layer();

        let expected_layer_one = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 0);
        let expected_layer_two = copy_of_nodes_in_layer(&graph, 1);

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 0, &expected_layer_one, "layer one");
        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
    });
}

#[test]
fn constraints_prevent_any_switch() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_formed_constraints_prevent_any_switch();

        let expected_layer_one = copy_of_nodes_in_layer(&graph, 0);
        let expected_layer_two = copy_of_nodes_in_layer(&graph, 1);

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 0, &expected_layer_one, "layer one");
        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
    });
}

#[test]
fn layout_unit_constraint_prevents_switch() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_nodes_in_different_layout_units_prevent_switch();

        let expected_layer_two = copy_of_nodes_in_layer(&graph, 1);

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 1, &expected_layer_two, "layer one");
    });
}

#[test]
fn one_node() {
    let graph = graph_one_node();
    let _ = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 0, 0);
}

#[test]
fn in_layer_switchable() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_in_layer_edges();

        let expected_order = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 1);

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 1, &expected_order, "layer one");
    });
}

#[test]
fn multiple_edges_between_same_nodes() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_multiple_edges_between_same_nodes();

        let expected_layer_one;
        let expected_layer_two;
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            expected_layer_one = copy_of_nodes_in_layer(&graph, 0);
            expected_layer_two = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 1);
        } else {
            expected_layer_one = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 0);
            expected_layer_two = copy_of_nodes_in_layer(&graph, 1);
        }

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 0, &expected_layer_one, "layer one");
        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
    });
}

#[test]
fn self_loops() {
    for_each_greedy_type(|greedy_type| {
        let graph = new_graph();
        let left_layer = make_layer(&graph);
        let right_layer = make_layer(&graph);

        let top_left = add_node_to_layer(&graph, &left_layer);
        let bottom_left = add_node_to_layer(&graph, &left_layer);
        let top_right = add_node_to_layer(&graph, &right_layer);
        let bottom_right = add_node_to_layer(&graph, &right_layer);

        let top_left_port = add_port_on_side(&top_left, PortSide::East);
        let bottom_left_port = add_port_on_side(&bottom_left, PortSide::East);
        set_up_ids(&graph);

        for layer in graph.lock().expect("graph lock").layers().clone() {
            for node in layer.lock().expect("layer lock").nodes().clone() {
                self_loop_on(&node, PortSide::East);
                self_loop_on(&node, PortSide::East);
                self_loop_on(&node, PortSide::East);
                self_loop_on(&node, PortSide::West);
                self_loop_on(&node, PortSide::West);
                self_loop_on(&node, PortSide::West);
            }
        }

        let top_right_port = add_port_on_side(&top_right, PortSide::West);
        let bottom_right_port = add_port_on_side(&bottom_right, PortSide::West);

        add_edge_between_ports(&top_left_port, &bottom_right_port);
        add_edge_between_ports(&bottom_left_port, &top_right_port);

        let expected_layer_one;
        let expected_layer_two;
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            expected_layer_one = copy_of_nodes_in_layer(&graph, 0);
            expected_layer_two = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 1);
        } else {
            expected_layer_one = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 0);
            expected_layer_two = copy_of_nodes_in_layer(&graph, 1);
        }

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 0, &expected_layer_one, "layer one");
        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
    });
}

#[test]
fn north_south_port_crossing() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_north_south_downward_crossing();

        let expected_two_sided = copy_of_nodes_in_layer(&graph, 0);
        let expected_one_sided = copy_of_switch_order_of_nodes_in_layer(&graph, 1, 2, 0);

        run_greedy_switcher(&graph, greedy_type);

        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert_layer_order(&graph, 0, &expected_one_sided, "layer one");
        } else {
            assert_layer_order(&graph, 0, &expected_two_sided, "layer one");
        }
    });
}

#[test]
fn more_complex() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_more_complex_three_layer();

        let expected_layer_two = copy_of_nodes_in_layer(&graph, 1);
        let expected_layer_three = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 2);

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
        assert_layer_order(&graph, 2, &expected_layer_three, "layer three");
    });
}

#[test]
fn switch_only_for_one_sided() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_switch_only_one_sided();

        let expected_one_sided = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 1);
        let expected_two_sided = copy_of_nodes_in_layer(&graph, 1);

        run_greedy_switcher(&graph, greedy_type);

        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert_layer_order(&graph, 1, &expected_one_sided, "layer two");
        } else {
            assert_layer_order(&graph, 1, &expected_two_sided, "layer two");
        }
    });
}

#[test]
fn does_not_worsen_cross_amount() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_which_could_be_worsened_by_switch();

        let expected_layer_one = copy_of_nodes_in_layer(&graph, 0);
        let expected_layer_two = copy_of_nodes_in_layer(&graph, 1);

        run_greedy_switcher(&graph, greedy_type);

        assert_layer_order(&graph, 0, &expected_layer_one, "layer one");
        assert_layer_order(&graph, 1, &expected_layer_two, "layer two");
    });
}

#[test]
fn switch_more_than_once() {
    for_each_greedy_type(|greedy_type| {
        let graph = new_graph();
        let left_nodes = add_nodes_to_layer(&graph, &make_layer(&graph), 2);
        let right_nodes = add_nodes_to_layer(&graph, &make_layer(&graph), 4);
        let left_top_port = add_port_on_side(&left_nodes[0], PortSide::East);
        let left_lower_port = add_port_on_side(&left_nodes[1], PortSide::East);
        let right_top_port = add_port_on_side(&right_nodes[0], PortSide::West);

        add_edge_between_ports(&left_lower_port, &right_top_port);
        east_west_edge_from_port(&left_lower_port, &right_nodes[2]);

        add_edge_between_ports(&left_top_port, &right_top_port);
        east_west_edge_from_port(&left_top_port, &right_nodes[1]);
        east_west_edge_from_port(&left_top_port, &right_nodes[3]);
        set_up_ids(&graph);

        let one_sided_first_layer = copy_of_nodes_in_layer(&graph, 0);
        let one_sided_first_switch = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 1);
        let one_sided_second_switch = get_copy_with_switched_order(2, 3, &one_sided_first_switch);
        let one_sided_third_switch = get_copy_with_switched_order(1, 2, &one_sided_second_switch);

        let two_sided_first_layer = copy_of_switch_order_of_nodes_in_layer(&graph, 0, 1, 0);
        let two_sided_first_switch = copy_of_switch_order_of_nodes_in_layer(&graph, 1, 2, 1);
        let two_sided_second_switch = get_copy_with_switched_order(0, 1, &two_sided_first_switch);

        run_greedy_switcher(&graph, greedy_type);

        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert_layer_order(&graph, 0, &one_sided_first_layer, "layer one");
            assert_layer_order(&graph, 1, &one_sided_third_switch, "layer two");
        } else {
            assert_layer_order(&graph, 0, &two_sided_first_layer, "layer one");
            assert_layer_order(&graph, 1, &two_sided_second_switch, "layer two");
        }
    });
}
