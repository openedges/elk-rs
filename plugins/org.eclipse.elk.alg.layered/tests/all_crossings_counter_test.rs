use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LNode, LPort, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::{
    init_initializables, AllCrossingsCounter, IInitializable,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSet;

fn add_node(
    graph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
    layer: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LayerRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef {
    let node = LNode::new(graph);
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    side: PortSide,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef {
    let port = LPort::new();
    if let Ok(mut port_guard) = port.lock() {
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(
    source: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
    target: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn add_in_layer_edge(
    node_one: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    node_two: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    side: PortSide,
) {
    let source = add_port(node_one, side);
    let target = add_port(node_two, side);
    connect(&source, &target);
}

fn set_fixed_order_constraint(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedOrder),
        );
    }
}

fn set_as_long_edge_dummy(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::LongEdge);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, None);
    }
}

fn add_north_south_edge(
    side: PortSide,
    node_with_ns_ports: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    north_south_dummy: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    node_with_east_west_ports: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    node_with_east_west_ports_is_origin: bool,
) {
    let ns_layer_index = node_with_ns_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock()
                .ok()
                .and_then(|layer_guard| layer_guard.index())
        })
        .unwrap_or(0);
    let other_layer_index = node_with_east_west_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock()
                .ok()
                .and_then(|layer_guard| layer_guard.index())
        })
        .unwrap_or(0);

    let normal_node_east_of_ns = other_layer_index < ns_layer_index;
    let direction = if normal_node_east_of_ns {
        PortSide::West
    } else {
        PortSide::East
    };
    let target_node_port_side = direction.opposed();

    let normal_node_port = add_port(node_with_east_west_ports, target_node_port_side);
    let dummy_node_port = add_port(north_south_dummy, direction);

    if node_with_east_west_ports_is_origin {
        connect(&normal_node_port, &dummy_node_port);
    } else {
        connect(&dummy_node_port, &normal_node_port);
    }

    if let Ok(mut dummy_guard) = north_south_dummy.lock() {
        dummy_guard.set_property(
            InternalProperties::IN_LAYER_LAYOUT_UNIT,
            Some(node_with_ns_ports.clone()),
        );
        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LNode(node_with_ns_ports.clone())),
        );
        dummy_guard.set_node_type(NodeType::NorthSouthPort);
    }

    let origin_port = add_port(node_with_ns_ports, side);
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(origin_port.clone())),
        );
    }
    if let Ok(mut origin_port_guard) = origin_port.lock() {
        origin_port_guard.set_property(
            InternalProperties::PORT_DUMMY,
            Some(north_south_dummy.clone()),
        );
    }

    if let Some(graph) = node_with_ns_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.graph())
    {
        if let Ok(mut graph_guard) = graph.lock() {
            let mut props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            props.insert(GraphProperties::NorthSouthPorts);
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
        }
    }
}

fn assign_ids(
    graph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) {
    if let Ok(graph_guard) = graph.lock() {
        for (layer_idx, layer) in graph_guard.layers().iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_idx as i32;
                for (node_idx, node) in layer_guard.nodes().iter().enumerate() {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.shape().graph_element().id = node_idx as i32;
                    }
                }
            }
        }
    }
}

#[test]
fn all_crossings_count_cross_form() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let top_left = add_node(&graph, &left_layer);
    let bottom_left = add_node(&graph, &left_layer);
    let top_right = add_node(&graph, &right_layer);
    let bottom_right = add_node(&graph, &right_layer);

    let top_left_port = add_port(&top_left, PortSide::East);
    let bottom_left_port = add_port(&bottom_left, PortSide::East);
    let top_right_port = add_port(&top_right, PortSide::West);
    let bottom_right_port = add_port(&bottom_right, PortSide::West);

    connect(&top_left_port, &bottom_right_port);
    connect(&bottom_left_port, &top_right_port);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 1);
}

#[test]
fn all_crossings_count_empty_graph() {
    let graph = LGraph::new();
    let order = graph.lock().expect("graph lock").to_node_array();

    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 0);
}

#[test]
fn all_crossings_count_multiple_edges_between_same_nodes() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let top_left = add_node(&graph, &left_layer);
    let bottom_left = add_node(&graph, &left_layer);
    let top_right = add_node(&graph, &right_layer);
    let bottom_right = add_node(&graph, &right_layer);

    let top_left_top_port = add_port(&top_left, PortSide::East);
    let top_left_bottom_port = add_port(&top_left, PortSide::East);
    let bottom_right_bottom_port = add_port(&bottom_right, PortSide::West);
    let bottom_right_top_port = add_port(&bottom_right, PortSide::West);
    connect(&top_left_top_port, &bottom_right_top_port);
    connect(&top_left_bottom_port, &bottom_right_bottom_port);

    let bottom_left_top_port = add_port(&bottom_left, PortSide::East);
    let bottom_left_bottom_port = add_port(&bottom_left, PortSide::East);
    let top_right_bottom_port = add_port(&top_right, PortSide::West);
    let top_right_top_port = add_port(&top_right, PortSide::West);
    connect(&bottom_left_top_port, &top_right_top_port);
    connect(&bottom_left_bottom_port, &top_right_bottom_port);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 4);
}

#[test]
fn all_crossings_switch_and_count_twice() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let top_left = add_node(&graph, &left_layer);
    let bottom_left = add_node(&graph, &left_layer);
    let top_right = add_node(&graph, &right_layer);
    let bottom_right = add_node(&graph, &right_layer);

    let top_left_port = add_port(&top_left, PortSide::East);
    let bottom_left_port = add_port(&bottom_left, PortSide::East);
    let top_right_port = add_port(&top_right, PortSide::West);
    let bottom_right_port = add_port(&bottom_right, PortSide::West);

    connect(&top_left_port, &bottom_right_port);
    connect(&bottom_left_port, &top_right_port);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 1);

    let mut switched_order = order.clone();
    switched_order[1].swap(0, 1);

    let mut switched_counter = AllCrossingsCounter::new(&switched_order);
    let mut switched_initables: [&mut dyn IInitializable; 1] = [&mut switched_counter];
    init_initializables(&mut switched_initables, &switched_order);
    assert_eq!(switched_counter.count_all_crossings(&switched_order), 0);
}

#[test]
fn all_crossings_count_in_layer_crossing() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let middle_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(middle_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_node = add_node(&graph, &left_layer);
    let middle_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &middle_layer)).collect();
    let right_node = add_node(&graph, &right_layer);

    let middle_to_right = add_port(&middle_nodes[1], PortSide::East);
    let right_from_middle = add_port(&right_node, PortSide::West);
    connect(&middle_to_right, &right_from_middle);

    let left_to_middle = add_port(&left_node, PortSide::East);
    let middle_from_left = add_port(&middle_nodes[1], PortSide::West);
    connect(&left_to_middle, &middle_from_left);

    add_in_layer_edge(&middle_nodes[0], &middle_nodes[2], PortSide::West);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 1);
}

#[test]
fn all_crossings_count_in_layer_crossing_and_switch() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let middle_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(middle_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_node = add_node(&graph, &left_layer);
    let middle_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &middle_layer)).collect();
    let right_node = add_node(&graph, &right_layer);

    let middle_to_right = add_port(&middle_nodes[1], PortSide::East);
    let right_from_middle = add_port(&right_node, PortSide::West);
    connect(&middle_to_right, &right_from_middle);

    let left_to_middle = add_port(&left_node, PortSide::East);
    let middle_from_left = add_port(&middle_nodes[1], PortSide::West);
    connect(&left_to_middle, &middle_from_left);

    add_in_layer_edge(&middle_nodes[0], &middle_nodes[2], PortSide::West);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 1);
}

#[test]
fn all_crossings_in_layer_crossings_on_far_left() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }

    let nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &layer)).collect();
    set_fixed_order_constraint(&nodes[1]);
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::West);
    add_in_layer_edge(&nodes[1], &nodes[2], PortSide::West);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 1);
}

#[test]
fn all_crossings_too_many_in_layer_crossings_with_the_old_method() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }

    let nodes: Vec<_> = (0..4).map(|_| add_node(&graph, &layer)).collect();
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::East);
    add_in_layer_edge(&nodes[2], &nodes[3], PortSide::East);

    assign_ids(&graph);

    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);

    assert_eq!(counter.count_all_crossings(&order), 0);
}

fn graph_with_north_south_crossing(
    side: PortSide,
    include_long_edge_dummy: bool,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let middle_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(middle_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_nodes: Vec<_> = (0..1).map(|_| add_node(&graph, &left_layer)).collect();
    let middle_nodes: Vec<_> = (0..4).map(|_| add_node(&graph, &middle_layer)).collect();
    let right_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &right_layer)).collect();

    let left_to_dummy = add_port(&left_nodes[0], PortSide::East);
    let dummy_from_left = add_port(&middle_nodes[2], PortSide::West);
    connect(&left_to_dummy, &dummy_from_left);

    let dummy_to_right = add_port(&middle_nodes[2], PortSide::East);
    let right_from_dummy = add_port(&right_nodes[2], PortSide::West);
    connect(&dummy_to_right, &right_from_dummy);
    if include_long_edge_dummy {
        set_as_long_edge_dummy(&middle_nodes[2]);
    }

    add_north_south_edge(
        side,
        &middle_nodes[3],
        &middle_nodes[0],
        &right_nodes[0],
        false,
    );
    add_north_south_edge(
        side,
        &middle_nodes[3],
        &middle_nodes[1],
        &right_nodes[1],
        false,
    );

    assign_ids(&graph);
    graph
}

#[test]
fn all_crossings_count_north_south_crossing() {
    let graph = graph_with_north_south_crossing(PortSide::North, false);
    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 0);
}

#[test]
fn all_crossings_count_northern_north_south_crossing() {
    let graph = graph_with_north_south_crossing(PortSide::South, false);
    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 0);
}

#[test]
fn all_crossings_north_south_dummy_edge_crossing() {
    let graph = graph_with_north_south_crossing(PortSide::North, true);
    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 2);
}

#[test]
fn all_crossings_one_node_is_long_edge_dummy() {
    let graph = graph_with_north_south_crossing(PortSide::North, true);
    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 2);
}

#[test]
fn all_crossings_one_node_is_long_edge_dummy_northern() {
    let graph = graph_with_north_south_crossing(PortSide::South, true);
    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 0);
}

fn graph_multiple_north_south_and_long_edge_dummies_on_both_sides(
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let middle_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(middle_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_nodes: Vec<_> = (0..2).map(|_| add_node(&graph, &left_layer)).collect();
    let middle_nodes: Vec<_> = (0..7).map(|_| add_node(&graph, &middle_layer)).collect();
    let right_nodes: Vec<_> = (0..6).map(|_| add_node(&graph, &right_layer)).collect();

    let left0_to_mid2 = add_port(&left_nodes[0], PortSide::East);
    let mid2_from_left = add_port(&middle_nodes[2], PortSide::West);
    connect(&left0_to_mid2, &mid2_from_left);

    let mid2_to_right2 = add_port(&middle_nodes[2], PortSide::East);
    let right2_from_mid2 = add_port(&right_nodes[2], PortSide::West);
    connect(&mid2_to_right2, &right2_from_mid2);

    let left1_to_mid4 = add_port(&left_nodes[1], PortSide::East);
    let mid4_from_left = add_port(&middle_nodes[4], PortSide::West);
    connect(&left1_to_mid4, &mid4_from_left);

    let mid4_to_right3 = add_port(&middle_nodes[4], PortSide::East);
    let right3_from_mid4 = add_port(&right_nodes[3], PortSide::West);
    connect(&mid4_to_right3, &right3_from_mid4);

    set_as_long_edge_dummy(&middle_nodes[2]);
    set_as_long_edge_dummy(&middle_nodes[4]);

    add_north_south_edge(
        PortSide::North,
        &middle_nodes[3],
        &middle_nodes[0],
        &right_nodes[0],
        false,
    );
    add_north_south_edge(
        PortSide::North,
        &middle_nodes[3],
        &middle_nodes[1],
        &right_nodes[1],
        false,
    );
    add_north_south_edge(
        PortSide::South,
        &middle_nodes[3],
        &middle_nodes[5],
        &right_nodes[4],
        false,
    );
    add_north_south_edge(
        PortSide::South,
        &middle_nodes[3],
        &middle_nodes[6],
        &right_nodes[5],
        false,
    );

    assign_ids(&graph);
    graph
}

#[test]
fn all_crossings_multiple_north_south_and_long_edge_dummies_on_both_sides() {
    let graph = graph_multiple_north_south_and_long_edge_dummies_on_both_sides();
    let order = graph.lock().expect("graph lock").to_node_array();
    let mut counter = AllCrossingsCounter::new(&order);
    let mut initables: [&mut dyn IInitializable; 1] = [&mut counter];
    init_initializables(&mut initables, &order);
    assert_eq!(counter.count_all_crossings(&order), 4);
}
