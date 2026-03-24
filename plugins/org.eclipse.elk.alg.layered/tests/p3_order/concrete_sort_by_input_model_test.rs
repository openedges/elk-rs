use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::SortByInputModelProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions, LongEdgeOrderingStrategy,
    OrderingStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    init_layered_metadata();
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    {
        let mut graph_guard = graph.lock();        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
            Some(OrderingStrategy::NodesAndEdges),
        );
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_LONG_EDGE_STRATEGY,
            Some(LongEdgeOrderingStrategy::Equal),
        );
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_PORT_MODEL_ORDER,
            Some(false),
        );
    }
    (graph, layers)
}

fn init_layered_metadata() {
    LayoutMetaDataService::get_instance()
        .register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn add_node(graph: &LGraphRef, layer: &LayerRef, model_order: i32) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.set_property(InternalProperties::MODEL_ORDER, Some(model_order));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock();        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect_with_model_order(source: &LPortRef, target: &LPortRef, model_order: i32) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge.lock()
        
        .set_property(InternalProperties::MODEL_ORDER, Some(model_order));
}

fn run_sorter_direct(graph: &LGraphRef) {
    let mut processor = SortByInputModelProcessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

fn assert_node_order(orders: [i32; 4]) {
    let (graph, layers) = graph_with_layers(1);
    let layer = layers[0].clone();
    let inserted = [
        add_node(&graph, &layer, orders[0]),
        add_node(&graph, &layer, orders[1]),
        add_node(&graph, &layer, orders[2]),
        add_node(&graph, &layer, orders[3]),
    ];

    run_sorter_direct(&graph);

    let nodes = layer.lock().nodes().clone();
    let mut sorted = inserted.to_vec();
    sorted.sort_by_key(|node| {
        node.lock()
            .get_property(InternalProperties::MODEL_ORDER)
            .unwrap_or(i32::MAX)
    });

    assert_eq!(nodes.len(), sorted.len());
    for (index, expected) in sorted.iter().enumerate() {
        assert!(
            Arc::ptr_eq(&nodes[index], expected),
            "node order mismatch at index {}",
            index
        );
    }
}

fn assert_port_order(edge_orders: [i32; 3], constraints: PortConstraints) {
    let (graph, layers) = graph_with_layers(2);
    let source_layer = layers[0].clone();
    let target_layer = layers[1].clone();

    let source = add_node(&graph, &source_layer, 0);
    source
        .lock()
        
        .set_property(LayeredOptions::PORT_CONSTRAINTS, Some(constraints));
    let ports = [
        add_port(&source, PortSide::East),
        add_port(&source, PortSide::East),
        add_port(&source, PortSide::East),
    ];

    let targets = [
        add_node(&graph, &target_layer, 0),
        add_node(&graph, &target_layer, 1),
        add_node(&graph, &target_layer, 2),
    ];
    let target_ports = [
        add_port(&targets[0], PortSide::West),
        add_port(&targets[1], PortSide::West),
        add_port(&targets[2], PortSide::West),
    ];

    connect_with_model_order(&ports[0], &target_ports[0], edge_orders[0]);
    connect_with_model_order(&ports[1], &target_ports[1], edge_orders[1]);
    connect_with_model_order(&ports[2], &target_ports[2], edge_orders[2]);

    run_sorter_direct(&graph);

    let sorted_ports = source.lock().ports().clone();
    if matches!(
        constraints,
        PortConstraints::FixedOrder | PortConstraints::FixedPos
    ) {
        assert!(Arc::ptr_eq(&sorted_ports[0], &ports[0]));
        assert!(Arc::ptr_eq(&sorted_ports[1], &ports[1]));
        assert!(Arc::ptr_eq(&sorted_ports[2], &ports[2]));
    } else {
        let mut expected = ports.to_vec();
        expected.sort_by_key(|port| {
            port.lock()
                .outgoing_edges()
                .first()
                .and_then(|edge| {
                    edge.lock().get_property(InternalProperties::MODEL_ORDER)
                })
                .unwrap_or(i32::MAX)
        });
        assert!(Arc::ptr_eq(&sorted_ports[0], &expected[0]));
        assert!(Arc::ptr_eq(&sorted_ports[1], &expected[1]));
        assert!(Arc::ptr_eq(&sorted_ports[2], &expected[2]));
    }
}

#[test]
fn sort_nodes_perm_0123() {
    assert_node_order([0, 1, 2, 3]);
}

#[test]
fn sort_nodes_perm_0132() {
    assert_node_order([0, 1, 3, 2]);
}

#[test]
fn sort_nodes_perm_0213() {
    assert_node_order([0, 2, 1, 3]);
}

#[test]
fn sort_nodes_perm_0231() {
    assert_node_order([0, 2, 3, 1]);
}

#[test]
fn sort_nodes_perm_0312() {
    assert_node_order([0, 3, 1, 2]);
}

#[test]
fn sort_nodes_perm_0321() {
    assert_node_order([0, 3, 2, 1]);
}

#[test]
fn sort_nodes_perm_1023() {
    assert_node_order([1, 0, 2, 3]);
}

#[test]
fn sort_nodes_perm_1203() {
    assert_node_order([1, 2, 0, 3]);
}

#[test]
fn sort_nodes_perm_1230() {
    assert_node_order([1, 2, 3, 0]);
}

#[test]
fn sort_nodes_perm_1302() {
    assert_node_order([1, 3, 0, 2]);
}

#[test]
fn sort_nodes_perm_2013() {
    assert_node_order([2, 0, 1, 3]);
}

#[test]
fn sort_nodes_perm_2301() {
    assert_node_order([2, 3, 0, 1]);
}

#[test]
fn sort_nodes_perm_3012() {
    assert_node_order([3, 0, 1, 2]);
}

#[test]
fn sort_nodes_perm_3210() {
    assert_node_order([3, 2, 1, 0]);
}

#[test]
fn sort_ports_by_edge_model_order_ascending() {
    assert_port_order([1, 2, 3], PortConstraints::Free);
}

#[test]
fn sort_ports_by_edge_model_order_descending() {
    assert_port_order([9, 3, 1], PortConstraints::Free);
}

#[test]
fn sort_ports_by_edge_model_order_mixed() {
    assert_port_order([5, 1, 3], PortConstraints::Free);
}

#[test]
fn sort_ports_keeps_order_when_fixed_order() {
    assert_port_order([9, 1, 5], PortConstraints::FixedOrder);
}

#[test]
fn sort_ports_keeps_order_when_fixed_pos() {
    assert_port_order([9, 1, 5], PortConstraints::FixedPos);
}
