#![allow(dead_code)]

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredMetaDataProvider, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::graph_info_holder::GraphInfoHolder;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::i_sweep_port_distributor::ISweepPortDistributor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::CrossMinType;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::layer_total_port_distributor::LayerTotalPortDistributor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::i_initializable::{
    init, IInitializable,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn set_up_graph(graph: &LGraphRef) {
    if let Some(mut graph_guard) = graph.lock_ok() {
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
    if let Some(mut graph_guard) = graph.lock_ok() {
        graph_guard.layers_mut().push(layer.clone());
    }
    layer
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    if let Some(mut node_guard) = node.lock_ok() {
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
    if let Some(mut port_guard) = port.lock_ok() {
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    if let Some(mut node_guard) = node.lock_ok() {
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

fn add_ports_on_side(node: &LNodeRef, count: usize, side: PortSide) -> Vec<LPortRef> {
    (0..count).map(|_| add_port_on_side(node, side)).collect()
}

fn connect(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn east_west_edge_from_to(left: &LNodeRef, right: &LNodeRef) {
    let left_port = add_port_on_side(left, PortSide::East);
    let right_port = add_port_on_side(right, PortSide::West);
    connect(&left_port, &right_port);
}

fn east_west_edge_from_port(left_port: &LPortRef, right: &LNodeRef) {
    let right_port = add_port_on_side(right, PortSide::West);
    connect(left_port, &right_port);
}

fn east_west_edge_to_port(left: &LNodeRef, right_port: &LPortRef) {
    let left_port = add_port_on_side(left, PortSide::East);
    connect(&left_port, right_port);
}

fn add_in_layer_edge(node_one: &LNodeRef, node_two: &LNodeRef, port_side: PortSide) {
    let port_one = add_port_on_side(node_one, port_side);
    let port_two = add_port_on_side(node_two, port_side);
    connect(&port_one, &port_two);
}

fn nested_graph(node: &LNodeRef) -> LGraphRef {
    if let Some(nested_graph) = node
        .lock_ok()
        .and_then(|node_guard| node_guard.nested_graph())
    {
        return nested_graph;
    }

    let nested_graph = LGraph::new();
    set_up_graph(&nested_graph);
    if let Some(mut graph_guard) = nested_graph.lock_ok() {
        graph_guard.set_parent_node(Some(node.clone()));
    }
    if let Some(mut node_guard) = node.lock_ok() {
        node_guard.set_nested_graph(Some(nested_graph.clone()));
    }
    nested_graph
}

fn add_external_port_dummy_node_to_layer(layer: &LayerRef, port: &LPortRef) -> LNodeRef {
    let graph = layer
        .lock_ok()
        .and_then(|layer_guard| layer_guard.graph())
        .expect("layer graph");
    let node = add_node(&graph, layer);
    let port_side = port
        .lock_ok()
        .map(|port_guard| port_guard.side())
        .unwrap_or(PortSide::Undefined);

    if let Some(mut node_guard) = node.lock_ok() {
        node_guard.set_node_type(NodeType::ExternalPort);
        node_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(port.clone())),
        );
        node_guard.set_property(InternalProperties::EXT_PORT_SIDE, Some(port_side));
    }

    if let Some(mut port_guard) = port.lock_ok() {
        port_guard.set_property(InternalProperties::PORT_DUMMY, Some(node.clone()));
        port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
    }

    if let Some(mut graph_guard) = graph.lock_ok() {
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
        .lock_ok()
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

fn add_north_south_edge(
    side: PortSide,
    node_with_ns_ports: &LNodeRef,
    north_south_dummy: &LNodeRef,
    node_with_east_west_ports: &LNodeRef,
    node_with_east_west_ports_is_origin: bool,
) {
    let ns_layer_index = node_with_ns_ports
        .lock_ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock_ok()
                .and_then(|layer_guard| layer_guard.index())
        })
        .unwrap_or(0);
    let other_layer_index = node_with_east_west_ports
        .lock_ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock_ok()
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
    let normal_node_port = add_port_on_side(node_with_east_west_ports, target_node_port_side);
    let dummy_node_port = add_port_on_side(north_south_dummy, direction);

    if node_with_east_west_ports_is_origin {
        connect(&normal_node_port, &dummy_node_port);
    } else {
        connect(&dummy_node_port, &normal_node_port);
    }

    if let Some(mut dummy_guard) = north_south_dummy.lock_ok() {
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

    let origin_port = add_port_on_side(node_with_ns_ports, side);
    if let Some(mut dummy_port_guard) = dummy_node_port.lock_ok() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(origin_port.clone())),
        );
    }
    if let Some(mut origin_port_guard) = origin_port.lock_ok() {
        origin_port_guard.set_property(
            InternalProperties::PORT_DUMMY,
            Some(north_south_dummy.clone()),
        );
    }

    let mut bary_assoc = vec![north_south_dummy.clone()];
    if let Some(mut node_guard) = node_with_ns_ports.lock_ok() {
        let existing = node_guard
            .get_property(InternalProperties::BARYCENTER_ASSOCIATES)
            .unwrap_or_default();
        if existing.is_empty() {
            node_guard.set_property(InternalProperties::BARYCENTER_ASSOCIATES, Some(bary_assoc));
        } else {
            bary_assoc.extend(existing);
            node_guard.set_property(InternalProperties::BARYCENTER_ASSOCIATES, Some(bary_assoc));
        }
    }

    if side == PortSide::North {
        if let Some(mut dummy_guard) = north_south_dummy.lock_ok() {
            let mut constraints = dummy_guard
                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                .unwrap_or_default();
            constraints.push(node_with_ns_ports.clone());
            dummy_guard.set_property(
                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                Some(constraints),
            );
        }
    } else if let Some(mut node_guard) = node_with_ns_ports.lock_ok() {
        let mut constraints = node_guard
            .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
            .unwrap_or_default();
        constraints.push(north_south_dummy.clone());
        node_guard.set_property(
            InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
            Some(constraints),
        );
    }
}

fn port_ptrs(node: &LNodeRef) -> Vec<usize> {
    let ports = node.lock().ports().clone();
    ports
        .iter()
        .map(|port| std::sync::Arc::as_ptr(port) as usize)
        .collect()
}

fn distribute_ports_in_complete_graph(graph: &LGraphRef) {
    let mut info = GraphInfoHolder::new(graph.clone(), CrossMinType::Barycenter);
    let node_order = graph.lock().to_node_array();
    for i in 0..node_order.len() {
        info.port_distributor()
            .distribute_ports_while_sweeping(&node_order, i, true);
    }
    for i in (0..node_order.len()).rev() {
        info.port_distributor()
            .distribute_ports_while_sweeping(&node_order, i, false);
    }
}

#[test]
fn distribute_ports_on_side_given_cross_on_western_side_removes_crossing() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_nodes = add_nodes(&graph, &make_layer(&graph), 2);
    let right_node = add_node(&graph, &make_layer(&graph));
    east_west_edge_from_to(&left_nodes[0], &right_node);
    east_west_edge_from_to(&left_nodes[1], &right_node);

    let original = port_ptrs(&right_node);
    let expected = vec![original[1], original[0]];

    distribute_ports_in_complete_graph(&graph);

    assert_eq!(port_ptrs(&right_node), expected);
}

#[test]
fn distribute_ports_of_graph_given_cross_on_both_sides_removes_crossing() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_nodes = add_nodes(&graph, &make_layer(&graph), 2);
    let middle_node = add_node(&graph, &make_layer(&graph));
    let right_nodes = add_nodes(&graph, &make_layer(&graph), 2);
    east_west_edge_from_to(&middle_node, &right_nodes[1]);
    east_west_edge_from_to(&middle_node, &right_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &middle_node);
    east_west_edge_from_to(&left_nodes[1], &middle_node);

    let original = port_ptrs(&middle_node);
    let expected = vec![original[1], original[0], original[3], original[2]];

    distribute_ports_in_complete_graph(&graph);

    assert_eq!(port_ptrs(&middle_node), expected);
}

#[test]
fn distribute_ports_of_graph_given_cross_on_eastern_side_removes_crossing() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_node = add_node(&graph, &make_layer(&graph));
    let right_nodes = add_nodes(&graph, &make_layer(&graph), 2);
    east_west_edge_from_to(&left_node, &right_nodes[1]);
    east_west_edge_from_to(&left_node, &right_nodes[0]);

    let original = port_ptrs(&left_node);
    let expected = vec![original[1], original[0]];

    distribute_ports_in_complete_graph(&graph);

    assert_eq!(port_ptrs(&left_node), expected);
}

#[test]
fn distribute_ports_of_graph_given_in_layer_edge_port_order_crossing_removes_it() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    add_node(&graph, &make_layer(&graph));
    let nodes = add_nodes(&graph, &make_layer(&graph), 3);
    add_in_layer_edge(&nodes[0], &nodes[2], PortSide::East);
    add_in_layer_edge(&nodes[1], &nodes[2], PortSide::East);

    let original = port_ptrs(&nodes[2]);
    let expected = vec![original[1], original[0]];

    distribute_ports_in_complete_graph(&graph);

    assert_eq!(port_ptrs(&nodes[2]), expected);
}

#[test]
fn distribute_ports_of_graph_given_north_south_port_order_crossing_switches() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_nodes = add_nodes(&graph, &make_layer(&graph), 3);
    let right_nodes = add_nodes(&graph, &make_layer(&graph), 2);

    add_north_south_edge(
        PortSide::North,
        &left_nodes[2],
        &left_nodes[1],
        &right_nodes[1],
        false,
    );
    add_north_south_edge(
        PortSide::North,
        &left_nodes[2],
        &left_nodes[0],
        &right_nodes[0],
        false,
    );

    let original = port_ptrs(&left_nodes[2]);
    let expected = vec![original[1], original[0]];

    distribute_ports_in_complete_graph(&graph);

    assert_eq!(port_ptrs(&left_nodes[2]), expected);
}

#[test]
fn distribute_ports_while_sweeping_given_simple_cross_removes_crossing() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_node = add_node(&graph, &make_layer(&graph));
    let right_node = add_node(&graph, &make_layer(&graph));
    east_west_edge_from_to(&left_node, &right_node);
    east_west_edge_from_to(&left_node, &right_node);

    let original = port_ptrs(&right_node);
    let expected = vec![original[1], original[0]];

    let node_order = graph.lock().to_node_array();
    let mut distributor = LayerTotalPortDistributor::new(node_order.len());
    let mut initables: [&mut dyn IInitializable; 1] = [&mut distributor];
    init(&mut initables, &node_order);
    distributor.distribute_ports_while_sweeping(&node_order, 1, true);

    assert_eq!(port_ptrs(&right_node), expected);
}
