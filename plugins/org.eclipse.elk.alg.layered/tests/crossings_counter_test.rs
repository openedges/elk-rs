use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, LPort, Layer};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::CrossingsCounter;
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
