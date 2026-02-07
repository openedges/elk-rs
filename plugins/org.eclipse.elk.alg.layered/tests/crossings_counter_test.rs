use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, LPort, Layer};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::CrossingsCounter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LEdge;

fn add_node(
    graph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
    layer: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LayerRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef {
    let node = LNode::new(graph);
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

fn add_ports_on_side(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    count: usize,
    side: PortSide,
) -> Vec<org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef> {
    (0..count).map(|_| add_port(node, side)).collect()
}

fn connect(
    source: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
    target: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

fn add_in_layer_edge(
    node_one: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    node_two: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    side: PortSide,
) {
    let port_one = add_port(node_one, side);
    let port_two = add_port(node_two, side);
    connect(&port_one, &port_two);
}

fn east_west_edge_from_to(
    left_node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    right_node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
) {
    let source = add_port(left_node, PortSide::East);
    let target = add_port(right_node, PortSide::West);
    connect(&source, &target);
}

fn east_west_edge_from_port(
    left_port: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef,
    right_node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
) {
    let target = add_port(right_node, PortSide::West);
    connect(left_port, &target);
}

fn add_self_loop(
    node: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LNodeRef,
    side: PortSide,
) {
    let port = add_port(node, side);
    connect(&port, &port);
}

fn assign_ids(
    graph: &org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LGraphRef,
) {
    let mut port_id = 0i32;
    if let Ok(graph_guard) = graph.lock() {
        for (layer_idx, layer) in graph_guard.layers().iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_idx as i32;
                for (node_idx, node) in layer_guard.nodes().iter().enumerate() {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.shape().graph_element().id = node_idx as i32;
                        for port in node_guard.ports_mut() {
                            if let Ok(mut port_guard) = port.lock() {
                                port_guard.shape().graph_element().id = port_id;
                                port_id += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn count_crossings_between_layers_simple() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_top = add_node(&graph, &left_layer);
    let left_bottom = add_node(&graph, &left_layer);
    let right_top = add_node(&graph, &right_layer);
    let right_bottom = add_node(&graph, &right_layer);

    let left_top_port = add_port(&left_top, PortSide::East);
    let left_bottom_port = add_port(&left_bottom, PortSide::East);
    let right_top_port = add_port(&right_top, PortSide::West);
    let right_bottom_port = add_port(&right_bottom, PortSide::West);

    connect(&left_top_port, &right_bottom_port);
    connect(&left_bottom_port, &right_top_port);

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 1);
}

#[test]
fn count_in_layer_crossings_simple() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }

    let nodes: Vec<_> = (0..5).map(|_| add_node(&graph, &layer)).collect();

    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::East);
    add_in_layer_edge(&nodes[1], &nodes[3], PortSide::East);
    add_in_layer_edge(&nodes[2], &nodes[4], PortSide::East);

    assign_ids(&graph);

    let layer_nodes = layer.lock().expect("layer lock").nodes().clone();
    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(
        counter.count_in_layer_crossings_on_side(&layer_nodes, PortSide::East),
        1
    );
}

#[test]
fn long_in_layer_crossings() {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }

    let nodes: Vec<_> = (0..5).map(|_| add_node(&graph, &layer)).collect();
    add_in_layer_edge(&nodes[0], &nodes[1], PortSide::East);
    add_in_layer_edge(&nodes[1], &nodes[3], PortSide::East);
    add_in_layer_edge(&nodes[2], &nodes[4], PortSide::East);

    assign_ids(&graph);

    let layer_nodes = layer.lock().expect("layer lock").nodes().clone();
    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(
        counter.count_in_layer_crossings_on_side(&layer_nodes, PortSide::East),
        1
    );
}

#[test]
fn count_crossings_between_layers_parallel_edges() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left = add_node(&graph, &left_layer);
    let right = add_node(&graph, &right_layer);

    let left_port_a = add_port(&left, PortSide::East);
    let right_port_a = add_port(&right, PortSide::West);
    connect(&left_port_a, &right_port_a);

    let left_port_b = add_port(&left, PortSide::East);
    let right_port_b = add_port(&right, PortSide::West);
    connect(&left_port_b, &right_port_b);

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 1);
}

#[test]
fn count_crossings_between_layers_cross_with_middle_edge() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &left_layer)).collect();
    let right_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &right_layer)).collect();

    let left_top = add_port(&left_nodes[0], PortSide::East);
    let left_mid = add_port(&left_nodes[1], PortSide::East);
    let left_bottom = add_port(&left_nodes[2], PortSide::East);

    let right_top = add_port(&right_nodes[0], PortSide::West);
    let right_mid = add_port(&right_nodes[1], PortSide::West);
    let right_bottom = add_port(&right_nodes[2], PortSide::West);

    connect(&left_top, &right_bottom);
    connect(&left_mid, &right_mid);
    connect(&left_bottom, &right_top);

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 3);
}

#[test]
fn count_crossings_between_layers_ignore_self_loops() {
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

    for _ in 0..3 {
        add_self_loop(&top_left, PortSide::East);
        add_self_loop(&bottom_left, PortSide::East);
        add_self_loop(&top_right, PortSide::West);
        add_self_loop(&bottom_right, PortSide::West);
    }

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 1);
}

#[test]
fn count_crossings_between_layers_into_same_port() {
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
    let bottom_right_port = add_port(&bottom_right, PortSide::West);
    connect(&top_left_port, &bottom_right_port);

    let bottom_left_first = add_port(&bottom_left, PortSide::East);
    let bottom_left_second = add_port(&bottom_left, PortSide::East);
    let top_right_port = add_port(&top_right, PortSide::West);

    connect(&bottom_left_first, &top_right_port);
    connect(&bottom_left_second, &top_right_port);

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 2);
}

#[test]
fn count_crossings_between_layers_cross_formed_multiple_edges_between_same_nodes() {
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

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 4);
}

#[test]
fn count_crossings_between_ports_given_western_crossings_only_counts_for_given_ports() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_nodes: Vec<_> = (0..2).map(|_| add_node(&graph, &left_layer)).collect();
    let right_nodes: Vec<_> = (0..2).map(|_| add_node(&graph, &right_layer)).collect();

    let left0_to_right1 = add_port(&left_nodes[0], PortSide::East);
    let right1_port0 = add_port(&right_nodes[1], PortSide::West);
    connect(&left0_to_right1, &right1_port0);

    let left1_to_right1 = add_port(&left_nodes[1], PortSide::East);
    let right1_port1 = add_port(&right_nodes[1], PortSide::West);
    connect(&left1_to_right1, &right1_port1);

    let left1_to_right0 = add_port(&left_nodes[1], PortSide::East);
    let right0_port0 = add_port(&right_nodes[0], PortSide::West);
    connect(&left1_to_right0, &right0_port0);

    assign_ids(&graph);

    let left_order = left_layer.lock().expect("layer lock").nodes().clone();
    let right_order = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    counter.init_for_counting_between(&left_order, &right_order);
    let pair = counter.count_crossings_between_ports_in_both_orders(&right1_port1, &right1_port0);
    assert_eq!(pair.first, 1);
}

#[test]
fn count_crossings_between_ports_given_crossings_on_eastern_side() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_node = add_node(&graph, &left_layer);
    let right_nodes: Vec<_> = (0..2).map(|_| add_node(&graph, &right_layer)).collect();

    let left_ports = add_ports_on_side(&left_node, 2, PortSide::East);
    let right_top = add_port(&right_nodes[0], PortSide::West);
    let right_bottom = add_port(&right_nodes[1], PortSide::West);
    connect(&left_ports[0], &right_bottom);
    connect(&left_ports[1], &right_top);

    assign_ids(&graph);

    let left_order = left_layer.lock().expect("layer lock").nodes().clone();
    let right_order = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    counter.init_for_counting_between(&left_order, &right_order);
    let pair = counter.count_crossings_between_ports_in_both_orders(&left_ports[0], &left_ports[1]);
    assert_eq!(pair.first, 1);
}

#[test]
fn count_crossings_between_ports_two_edges_into_same_port() {
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
    let bottom_right_port = add_port(&bottom_right, PortSide::West);
    connect(&top_left_port, &bottom_right_port);

    let bottom_left_port = add_port(&bottom_left, PortSide::East);
    let top_right_port = add_port(&top_right, PortSide::West);
    connect(&bottom_left_port, &top_right_port);
    connect(&bottom_left_port, &top_right_port);

    assign_ids(&graph);

    let left_order = left_layer.lock().expect("layer lock").nodes().clone();
    let right_order = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    counter.init_for_counting_between(&left_order, &right_order);
    let pair = counter.count_crossings_between_ports_in_both_orders(&bottom_left_port, &top_left_port);
    assert_eq!(pair.first, 2);
}

#[test]
fn count_crossings_between_layers_fixed_port_order() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left = add_node(&graph, &left_layer);
    let right = add_node(&graph, &right_layer);

    if let Ok(mut right_guard) = right.lock() {
        right_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedOrder),
        );
    }

    let left_port_top = add_port(&left, PortSide::East);
    let left_port_bottom = add_port(&left, PortSide::East);
    let right_port_top = add_port(&right, PortSide::West);
    let right_port_bottom = add_port(&right, PortSide::West);
    connect(&left_port_top, &right_port_bottom);
    connect(&left_port_bottom, &right_port_top);

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let right_nodes = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &right_nodes), 0);
}

#[test]
fn count_crossings_between_layers_more_complex_three_layer_graph() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let middle_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(middle_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &left_layer)).collect();
    let middle_nodes: Vec<_> = (0..2).map(|_| add_node(&graph, &middle_layer)).collect();
    let right_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &right_layer)).collect();

    let left0 = add_port(&left_nodes[0], PortSide::East);
    let left1 = add_port(&left_nodes[1], PortSide::East);
    let left2 = add_port(&left_nodes[2], PortSide::East);
    let middle0_a = add_port(&middle_nodes[0], PortSide::West);
    let middle0_b = add_port(&middle_nodes[0], PortSide::West);
    let middle1_a = add_port(&middle_nodes[1], PortSide::West);
    connect(&left0, &middle1_a);
    connect(&left1, &middle0_a);
    connect(&left2, &middle0_b);

    let middle0_east = add_port(&middle_nodes[0], PortSide::East);
    let middle1_east = add_port(&middle_nodes[1], PortSide::East);
    let right0 = add_port(&right_nodes[0], PortSide::West);
    let right1 = add_port(&right_nodes[1], PortSide::West);
    let right2 = add_port(&right_nodes[2], PortSide::West);
    connect(&middle0_east, &right0);
    connect(&middle1_east, &right2);
    connect(&middle1_east, &right1);

    assign_ids(&graph);

    let left_nodes = left_layer.lock().expect("layer lock").nodes().clone();
    let middle_nodes = middle_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    assert_eq!(counter.count_crossings_between_layers(&left_nodes, &middle_nodes), 3);
}

#[test]
fn counting_two_different_graphs_does_not_interfere_with_each_other() {
    let graph = LGraph::new();
    let left_layer = Layer::new(&graph);
    let right_layer = Layer::new(&graph);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(left_layer.clone());
        graph_guard.layers_mut().push(right_layer.clone());
    }

    let left_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &left_layer)).collect();
    let right_nodes: Vec<_> = (0..3).map(|_| add_node(&graph, &right_layer)).collect();
    let left_node = left_nodes[1].clone();
    let left_ports = add_ports_on_side(&left_node, 2, PortSide::East);

    east_west_edge_from_to(&left_nodes[2], &right_nodes[1]);
    east_west_edge_from_port(&left_ports[0], &right_nodes[1]);
    east_west_edge_from_port(&left_ports[1], &right_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[0]);

    assign_ids(&graph);

    let left_order = left_layer.lock().expect("layer lock").nodes().clone();
    let right_order = right_layer.lock().expect("layer lock").nodes().clone();

    let mut counter = CrossingsCounter::new(Vec::new());
    counter.init_for_counting_between(&left_order, &right_order);
    let pair = counter.count_crossings_between_ports_in_both_orders(&left_ports[0], &left_ports[1]);
    assert_eq!(pair.first, 1);

    counter.switch_ports(&left_ports[0], &left_ports[1]);
    let switched_pair =
        counter.count_crossings_between_ports_in_both_orders(&left_ports[1], &left_ports[0]);
    assert_eq!(switched_pair.first, 0);
}
