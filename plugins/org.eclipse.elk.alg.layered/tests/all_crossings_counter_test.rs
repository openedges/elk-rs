use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNode, LPort, Layer};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::{
    init_initializables, AllCrossingsCounter, IInitializable,
};
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

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
