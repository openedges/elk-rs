use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredMetaDataProvider, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::{
    GreedyPortDistributor, ISweepPortDistributor,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::i_initializable::{
    init, IInitializable,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

fn set_up_graph(graph: &LGraphRef) {
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
        graph_guard.set_property(InternalProperties::RANDOM, Some(Random::new(0)));
    }
}

fn make_layer(graph: &LGraphRef) -> LayerRef {
    let layer = Layer::new(graph);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.layers_mut().push(layer.clone());
    }
    layer
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::Normal);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_nodes(graph: &LGraphRef, layer: &LayerRef, count: usize) -> Vec<LNodeRef> {
    (0..count).map(|_| add_node(graph, layer)).collect()
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

fn add_ports_on_side(node: &LNodeRef, count: usize, side: PortSide) -> Vec<LPortRef> {
    (0..count).map(|_| add_port_on_side(node, side)).collect()
}

fn connect(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn connect_nodes_east_west(left: &LNodeRef, right: &LNodeRef) {
    let left_port = add_port_on_side(left, PortSide::East);
    let right_port = add_port_on_side(right, PortSide::West);
    connect(&left_port, &right_port);
}

fn connect_port_to_node(left_port: &LPortRef, right: &LNodeRef) {
    let right_port = add_port_on_side(right, PortSide::West);
    connect(left_port, &right_port);
}

fn connect_node_to_port(left: &LNodeRef, port: &LPortRef) {
    let left_port = add_port_on_side(left, PortSide::East);
    connect(&left_port, port);
}

fn port_ids(node: &LNodeRef) -> Vec<i32> {
    let ports = node.lock().expect("node lock").ports().clone();
    ports
        .iter()
        .filter_map(|port| port.lock().ok())
        .map(|mut port_guard| port_guard.shape().graph_element().id)
        .collect()
}

fn port_ptrs(node: &LNodeRef) -> Vec<usize> {
    let ports = node.lock().expect("node lock").ports().clone();
    ports.iter().map(|port| Arc::as_ptr(port) as usize).collect()
}

fn port_ptrs_from_ports(ports: &[LPortRef]) -> Vec<usize> {
    ports.iter().map(|port| Arc::as_ptr(port) as usize).collect()
}

fn prepare_distributor(graph: &LGraphRef, distributor: &mut GreedyPortDistributor) -> Vec<Vec<LNodeRef>> {
    let node_order = graph.lock().expect("graph lock").to_node_array();
    let mut initables: [&mut dyn IInitializable; 1] = [distributor];
    init(&mut initables, &node_order);
    node_order
}

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn nested_graph(node: &LNodeRef) -> LGraphRef {
    if let Some(nested_graph) = node.lock().ok().and_then(|node_guard| node_guard.nested_graph()) {
        return nested_graph;
    }

    let nested_graph = LGraph::new();
    set_up_graph(&nested_graph);
    if let Ok(mut graph_guard) = nested_graph.lock() {
        graph_guard.set_parent_node(Some(node.clone()));
    }
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_nested_graph(Some(nested_graph.clone()));
    }
    nested_graph
}

fn add_external_port_dummy_node_to_layer(layer: &LayerRef, port: &LPortRef) -> LNodeRef {
    let graph = layer.lock().ok().and_then(|layer_guard| layer_guard.graph()).expect("layer graph");
    let node = add_node(&graph, layer);
    let port_side = port.lock().ok().map(|port_guard| port_guard.side()).unwrap_or(PortSide::Undefined);

    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::ExternalPort);
        node_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(port.clone())));
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

fn add_external_port_dummies_to_layer(layer: &LayerRef, ports: &[LPortRef]) -> Vec<LNodeRef> {
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
        let port_index = if side == PortSide::East { i } else { ports.len() - 1 - i };
        nodes.push(add_external_port_dummy_node_to_layer(layer, &ports[port_index]));
    }
    nodes
}

#[test]
fn distribute_ports_cross_on_western_side_removes_crossing() {
    init_layered_options();
    let graph = LGraph::new();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes(&graph, &left_layer, 2);
    let right_node = add_node(&graph, &right_layer);

    connect_nodes_east_west(&left_nodes[0], &right_node);
    connect_nodes_east_west(&left_nodes[1], &right_node);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ids(&right_node);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 1, true);
    let after = port_ids(&right_node);

    assert!(improved);
    assert_eq!(after, vec![original[1], original[0]]);
}

#[test]
fn distribute_ports_no_ports_on_right_side_no_change() {
    init_layered_options();
    let graph = LGraph::new();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes(&graph, &left_layer, 2);
    let middle_node = add_node(&graph, &middle_layer);
    let _right_nodes = add_nodes(&graph, &right_layer, 2);

    connect_nodes_east_west(&left_nodes[0], &middle_node);
    connect_nodes_east_west(&left_nodes[1], &middle_node);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ids(&middle_node);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 1, false);
    let after = port_ids(&middle_node);

    assert!(!improved);
    assert_eq!(after, original);
}

#[test]
fn distribute_ports_multiple_crossings_on_western_side_removes_crossing() {
    init_layered_options();
    let graph = LGraph::new();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes(&graph, &left_layer, 3);
    let right_node = add_node(&graph, &right_layer);

    connect_nodes_east_west(&left_nodes[0], &right_node);
    connect_nodes_east_west(&left_nodes[2], &right_node);
    connect_nodes_east_west(&left_nodes[1], &right_node);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ids(&right_node);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 1, true);
    let after = port_ids(&right_node);

    assert!(improved);
    assert_eq!(after, vec![original[2], original[0], original[1]]);
}

#[test]
fn distribute_ports_crossings_on_eastern_side_removes_them() {
    init_layered_options();
    let graph = LGraph::new();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes(&graph, &left_layer, 1);
    let right_nodes = add_nodes(&graph, &right_layer, 2);

    connect_node_to_port(&left_nodes[0], &add_port_on_side(&right_nodes[1], PortSide::West));
    connect_node_to_port(&left_nodes[0], &add_port_on_side(&right_nodes[0], PortSide::West));

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ids(&left_nodes[0]);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ids(&left_nodes[0]);

    assert!(improved);
    assert_eq!(after, vec![original[1], original[0]]);
}

#[test]
fn distribute_ports_fixed_order_no_change() {
    init_layered_options();
    let graph = LGraph::new();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes(&graph, &left_layer, 1);
    let right_nodes = add_nodes(&graph, &right_layer, 2);

    if let Ok(mut node_guard) = left_nodes[0].lock() {
        node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(PortConstraints::FixedOrder));
    }

    connect_node_to_port(&left_nodes[0], &add_port_on_side(&right_nodes[1], PortSide::West));
    connect_node_to_port(&left_nodes[0], &add_port_on_side(&right_nodes[0], PortSide::West));

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ids(&left_nodes[0]);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ids(&left_nodes[0]);

    assert!(!improved);
    assert_eq!(after, original);
}

#[test]
fn distribute_ports_no_change_in_ordered_case() {
    init_layered_options();
    let graph = LGraph::new();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_nodes = add_nodes(&graph, &left_layer, 1);
    let right_nodes = add_nodes(&graph, &right_layer, 2);

    connect_nodes_east_west(&left_nodes[0], &right_nodes[0]);
    connect_nodes_east_west(&left_nodes[0], &right_nodes[1]);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    assert!(!improved);
}

#[test]
fn distribute_ports_double_cross_between_compound_and_non_compound_nodes_switches_ports() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = add_nodes(&graph, &right_layer, 2);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);

    connect_port_to_node(&left_outer_ports[0], &right_nodes[1]);
    connect_port_to_node(&left_outer_ports[0], &right_nodes[1]);
    connect_port_to_node(&left_outer_ports[1], &right_nodes[0]);

    let left_inner_graph = nested_graph(&left_outer_node);
    let left_inner_nodes = add_nodes(&left_inner_graph, &make_layer(&left_inner_graph), 2);
    let left_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&make_layer(&left_inner_graph), &left_outer_ports);
    connect_nodes_east_west(&left_inner_nodes[0], &left_inner_dummy_nodes[0]);
    connect_nodes_east_west(&left_inner_nodes[1], &left_inner_dummy_nodes[1]);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ids(&left_outer_node);
    let improved = distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ids(&left_outer_node);

    assert!(improved);
    assert_eq!(after, vec![original[1], original[0]]);
}

#[test]
fn distribute_ports_single_cross_between_compound_and_non_compound_nodes_no_switch() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = add_nodes(&graph, &right_layer, 2);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);

    connect_port_to_node(&left_outer_ports[0], &right_nodes[1]);
    connect_port_to_node(&left_outer_ports[1], &right_nodes[0]);

    let left_inner_graph = nested_graph(&left_outer_node);
    let left_inner_nodes = add_nodes(&left_inner_graph, &make_layer(&left_inner_graph), 2);
    let left_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&make_layer(&left_inner_graph), &left_outer_ports);
    connect_nodes_east_west(&left_inner_nodes[0], &left_inner_dummy_nodes[0]);
    connect_nodes_east_west(&left_inner_nodes[1], &left_inner_dummy_nodes[1]);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let expected = port_ptrs_from_ports(&left_outer_ports);
    distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ptrs(&left_outer_node);
    assert_eq!(after, expected);
}

#[test]
fn distribute_ports_more_hierarchical_nodes_no_switch() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = add_nodes(&graph, &right_layer, 3);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 3, PortSide::East);

    connect_port_to_node(&left_outer_ports[0], &right_nodes[1]);
    connect_port_to_node(&left_outer_ports[1], &right_nodes[0]);
    connect_port_to_node(&left_outer_ports[2], &right_nodes[2]);

    let left_inner_graph = nested_graph(&left_outer_node);
    let left_inner_nodes = add_nodes(&left_inner_graph, &make_layer(&left_inner_graph), 3);
    let left_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&make_layer(&left_inner_graph), &left_outer_ports);
    connect_nodes_east_west(&left_inner_nodes[0], &left_inner_dummy_nodes[0]);
    connect_nodes_east_west(&left_inner_nodes[1], &left_inner_dummy_nodes[1]);
    connect_nodes_east_west(&left_inner_nodes[2], &left_inner_dummy_nodes[2]);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let expected = port_ptrs_from_ports(&left_outer_ports);
    distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ptrs(&left_outer_node);
    assert_eq!(after, expected);
}

#[test]
fn distribute_ports_more_hierarchical_nodes_variant_no_switch() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = add_nodes(&graph, &right_layer, 3);
    let left_outer_ports = add_ports_on_side(&left_outer_node, 3, PortSide::East);

    connect_port_to_node(&left_outer_ports[0], &right_nodes[1]);
    connect_port_to_node(&left_outer_ports[1], &right_nodes[0]);
    connect_port_to_node(&left_outer_ports[2], &right_nodes[2]);

    let left_inner_graph = nested_graph(&left_outer_node);
    let left_inner_node = add_node(&left_inner_graph, &make_layer(&left_inner_graph));
    let right_inner_nodes = add_nodes(&left_inner_graph, &make_layer(&left_inner_graph), 3);
    let dummy_nodes = add_external_port_dummies_to_layer(&make_layer(&left_inner_graph), &left_outer_ports);

    connect_nodes_east_west(&left_inner_node, &right_inner_nodes[2]);
    connect_nodes_east_west(&right_inner_nodes[0], &dummy_nodes[0]);
    connect_nodes_east_west(&right_inner_nodes[1], &dummy_nodes[1]);
    connect_nodes_east_west(&right_inner_nodes[2], &dummy_nodes[2]);

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ptrs(&left_outer_node);
    distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ptrs(&left_outer_node);
    assert_eq!(after, original);
}

#[test]
fn distribute_ports_two_nodes_in_one_layer_no_switch() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);

    for _ in 0..2 {
        let left_nodes = add_nodes(&graph, &left_layer, 1);
        let right_nodes = add_nodes(&graph, &right_layer, 2);
        connect_nodes_east_west(&left_nodes[0], &right_nodes[1]);
        connect_nodes_east_west(&left_nodes[0], &right_nodes[0]);
    }

    let left_outer_node = left_layer
        .lock()
        .ok()
        .and_then(|layer_guard| layer_guard.nodes().first().cloned())
        .expect("left layer node");

    let mut distributor = GreedyPortDistributor::new();
    let node_order = prepare_distributor(&graph, &mut distributor);
    let original = port_ptrs(&left_outer_node);
    let expected = vec![original[1], original[0]];
    distributor.distribute_ports_while_sweeping(&node_order, 0, false);
    let after = port_ptrs(&left_outer_node);
    assert_eq!(after, expected);
}
