use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::greedyswitch::NorthSouthEdgeNeighbouringNodeCrossingsCounter;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

fn init_layout() {
    LayoutMetaDataService::get_instance();
}

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

fn set_as_long_edge_dummy(node: &LNodeRef) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::LongEdge);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, None);
    }
}

fn set_node_type_long_edge(node: &LNodeRef) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::LongEdge);
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

    let mut bary_assoc = vec![north_south_dummy.clone()];
    if let Ok(mut node_guard) = node_with_ns_ports.lock() {
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

fn layer_nodes(graph: &LGraphRef, layer_index: usize) -> Vec<LNodeRef> {
    graph
        .lock()
        .expect("graph lock")
        .to_node_array()
        .get(layer_index)
        .cloned()
        .expect("layer index")
}

fn count_crossings(layer: &[LNodeRef], upper_index: usize, lower_index: usize) -> (i32, i32) {
    let mut counter = NorthSouthEdgeNeighbouringNodeCrossingsCounter::new(layer);
    counter.count_crossings(&layer[upper_index], &layer[lower_index]);
    (counter.upper_lower_crossings(), counter.lower_upper_crossings())
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

fn graph_north_south_downward_crossing() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    add_north_south_edge(PortSide::South, &left_nodes[0], &left_nodes[2], &right_nodes[1], false);
    add_north_south_edge(PortSide::South, &left_nodes[0], &left_nodes[1], &right_nodes[0], false);
    set_fixed_order_constraint(&left_nodes[0]);
    graph
}

fn graph_north_south_upward_crossing() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    add_north_south_edge(PortSide::North, &left_nodes[2], &left_nodes[1], &right_nodes[1], false);
    add_north_south_edge(PortSide::North, &left_nodes[2], &left_nodes[0], &right_nodes[0], false);
    set_fixed_order_constraint(&left_nodes[2]);
    graph
}

fn graph_southern_north_south_dummy_edge_crossing() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_node, &middle_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[0]);
    set_as_long_edge_dummy(&middle_nodes[1]);

    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &right_nodes[1], true);
    graph
}

fn graph_northern_north_south_dummy_edge_crossing() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_node, &middle_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[1]);
    set_as_long_edge_dummy(&middle_nodes[1]);

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &right_nodes[0], true);
    graph
}

fn graph_southern_north_south_edges_from_east_and_west_no_crossings() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[0]);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[1], &right_node, false);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &left_node, true);
    graph
}

fn graph_northern_north_south_edges_from_east_and_west_no_crossings() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[2]);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &left_node, true);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &right_node, false);
    graph
}

fn graph_northern_north_south_edges_from_east_and_west_no_crossings_upper_edge_east() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[2]);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &left_node, true);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &right_node, false);
    graph
}

fn graph_southern_north_south_edges_both_to_east() -> LGraphRef {
    let graph = new_graph();
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&middle_nodes[0]);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[1], &right_nodes[0], false);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &right_nodes[1], false);
    graph
}

fn graph_where_layout_unit_prevents_switch() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 4);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&left_nodes[0]);
    set_fixed_order_constraint(&left_nodes[3]);

    add_north_south_edge(PortSide::South, &left_nodes[0], &left_nodes[1], &right_nodes[1], false);
    add_north_south_edge(PortSide::North, &left_nodes[3], &left_nodes[2], &right_nodes[0], false);
    graph
}

fn graph_north_south_edges_from_east_and_west_and_cross() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[0]);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[1], &left_node, true);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &right_node, false);
    graph
}

fn graph_south_port_on_normal_node_below_long_edge_dummy() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_node, &middle_nodes[0]);
    east_west_edge_from_to(&middle_nodes[0], &right_nodes[0]);
    set_node_type_long_edge(&middle_nodes[0]);

    add_north_south_edge(PortSide::South, &middle_nodes[1], &middle_nodes[2], &right_nodes[1], false);
    graph
}

fn graph_north_port_on_normal_node_above_long_edge_dummy() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_node, &middle_nodes[2]);
    east_west_edge_from_to(&middle_nodes[2], &right_nodes[1]);
    set_node_type_long_edge(&middle_nodes[2]);

    add_north_south_edge(PortSide::North, &middle_nodes[1], &middle_nodes[0], &right_nodes[0], false);
    graph
}

fn graph_north_south_southern_two_western_edges() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);

    set_fixed_order_constraint(&middle_nodes[0]);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[1], &left_nodes[0], true);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &left_nodes[1], true);
    graph
}

fn graph_north_south_southern_western_port_to_east_and_eastern_port_to_west() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[0]);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &left_node, true);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[1], &right_node, false);
    graph
}

fn graph_north_south_northern_western_edges() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);

    set_fixed_order_constraint(&middle_nodes[2]);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &left_nodes[1], true);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &left_nodes[0], true);
    graph
}

fn graph_north_south_northern_eastern_port_to_west_western_port_to_east() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[2]);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &right_node, false);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &left_node, true);
    graph
}

fn graph_multiple_north_south_and_long_edge_dummies_on_both_sides() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 7);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 6);

    east_west_edge_from_to(&left_nodes[0], &middle_nodes[2]);
    east_west_edge_from_to(&middle_nodes[2], &right_nodes[2]);
    east_west_edge_from_to(&left_nodes[1], &middle_nodes[4]);
    east_west_edge_from_to(&middle_nodes[4], &right_nodes[4]);

    set_as_long_edge_dummy(&middle_nodes[2]);
    set_as_long_edge_dummy(&middle_nodes[4]);

    add_north_south_edge(PortSide::North, &middle_nodes[3], &middle_nodes[0], &right_nodes[0], false);
    add_north_south_edge(PortSide::North, &middle_nodes[3], &middle_nodes[1], &right_nodes[1], false);
    add_north_south_edge(PortSide::South, &middle_nodes[3], &middle_nodes[5], &right_nodes[4], false);
    add_north_south_edge(PortSide::South, &middle_nodes[3], &middle_nodes[6], &right_nodes[5], false);
    graph
}

fn graph_long_edge_dummy_and_normal_node_with_unused_ports_on_southern_side() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 2);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[0]);
    east_west_edge_from_to(&left_node, &middle_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_node);
    set_as_long_edge_dummy(&middle_nodes[1]);

    add_port_on_side(&middle_nodes[0], PortSide::South);
    add_port_on_side(&middle_nodes[0], PortSide::South);
    graph
}

fn graph_long_edge_dummy_and_normal_node_with_unused_ports_on_northern_side() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 2);
    let right_node = add_node_to_layer(&graph, &right_layer);

    east_west_edge_from_to(&left_node, &middle_nodes[0]);
    east_west_edge_from_to(&middle_nodes[0], &right_node);
    set_node_type_long_edge(&middle_nodes[0]);

    add_port_on_side(&middle_nodes[1], PortSide::North);
    add_port_on_side(&middle_nodes[1], PortSide::North);
    graph
}

#[test]
fn no_north_south_node() {
    init_layout();
    let graph = graph_cross_formed();
    let layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);
}

#[test]
fn southern_north_south_node_crossing() {
    init_layout();
    let graph = graph_north_south_downward_crossing();
    let layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn northern_north_south_node_crossings() {
    init_layout();
    let graph = graph_north_south_upward_crossing();
    let layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn one_node_is_long_edge_dummy() {
    init_layout();
    let graph = graph_southern_north_south_dummy_edge_crossing();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);

    layer.swap(1, 2);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);
}

#[test]
fn one_node_is_long_edge_dummy_northern() {
    init_layout();
    let graph = graph_northern_north_south_dummy_edge_crossing();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);

    layer.swap(0, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);
}

#[test]
fn with_normal_node() {
    init_layout();
    let graph = graph_north_south_downward_crossing();
    let layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);
}

#[test]
fn north_south_edges_come_from_both_sides_dont_cross() {
    init_layout();
    let graph = graph_southern_north_south_edges_from_east_and_west_no_crossings();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);

    let graph = graph_northern_north_south_edges_from_east_and_west_no_crossings();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);
}

#[test]
fn southern_north_south_edges_both_to_east() {
    init_layout();
    let graph = graph_southern_north_south_edges_both_to_east();
    let layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);
}

#[test]
fn crossings_with_north_south_ports_belonging_to_different_nodes_should_not_be_counted() {
    init_layout();
    let graph = graph_where_layout_unit_prevents_switch();
    let layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);
}

#[test]
fn north_south_edges_come_from_both_sides_do_cross() {
    init_layout();
    let graph = graph_north_south_edges_from_east_and_west_and_cross();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 1);
}

#[test]
fn switch_nodes_and_recount() {
    init_layout();
    let graph = graph_north_south_upward_crossing();
    let mut layer = layer_nodes(&graph, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);

    layer.swap(0, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);
}

#[test]
fn south_port_on_normal_node_below_long_edge_dummy() {
    init_layout();
    let graph = graph_south_port_on_normal_node_below_long_edge_dummy();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);

    layer.swap(0, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn north_port_on_normal_node_above_long_edge_dummy() {
    init_layout();
    let graph = graph_north_port_on_normal_node_above_long_edge_dummy();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);

    layer.swap(1, 2);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn southern_two_western_edges() {
    init_layout();
    let graph = graph_north_south_southern_two_western_edges();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);

    layer.swap(1, 2);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);
}

#[test]
fn southern_western_port_to_east_and_eastern_port_to_west() {
    init_layout();
    let graph = graph_north_south_southern_western_port_to_east_and_eastern_port_to_west();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 1);

    layer.swap(1, 2);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 1);
}

#[test]
fn northern_both_edges_western() {
    init_layout();
    let graph = graph_north_south_northern_western_edges();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 1);

    layer.swap(0, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn northern_eastern_port_to_west_western_port_to_east() {
    init_layout();
    let graph = graph_north_south_northern_eastern_port_to_west_western_port_to_east();
    let mut layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 1);

    layer.swap(0, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 1);
}

#[test]
fn normal_nodes_north_south_edges_have_crossings_to_long_edge_dummy() {
    init_layout();
    let graph = graph_northern_north_south_dummy_edge_crossing();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);

    let graph = graph_southern_north_south_dummy_edge_crossing();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
    let (upper_lower, lower_upper) = count_crossings(&layer, 1, 2);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn normal_nodes_north_south_edges_have_crossings_to_long_edge_dummy_on_both_sides() {
    init_layout();
    let graph = graph_multiple_north_south_and_long_edge_dummies_on_both_sides();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 2, 3);
    assert_eq!(upper_lower, 2);
    assert_eq!(lower_upper, 2);
}

#[test]
fn ignores_unconnected_ports_for_normal_node_and_long_edge_dummies() {
    init_layout();
    let graph = graph_long_edge_dummy_and_normal_node_with_unused_ports_on_southern_side();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, _lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);

    let graph = graph_long_edge_dummy_and_normal_node_with_unused_ports_on_northern_side();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, _lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
}

#[test]
fn one_edge_west_one_edge_east_dont_cross() {
    init_layout();
    let graph = graph_northern_north_south_edges_from_east_and_west_no_crossings();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);
}

#[test]
fn one_edge_east_one_edge_west_dont_cross() {
    init_layout();
    let graph = graph_northern_north_south_edges_from_east_and_west_no_crossings_upper_edge_east();
    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 0);
    assert_eq!(lower_upper, 0);
}

#[test]
fn given_polyline_routing_when_more_than_one_edge_into_ns_node_counts_these_too() {
    init_layout();
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&middle_nodes[2]);

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &right_nodes[0], false);
    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &left_node, true);

    let middle_node_port = middle_nodes[1]
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports().get(0).cloned())
        .expect("north/south dummy port");
    east_west_edge_from_port(&middle_node_port, &right_nodes[1]);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Polyline));
    }

    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 2);
    assert_eq!(lower_upper, 1);
}

#[test]
fn given_multiple_edges_in_one_ns_node_counts_crossings() {
    init_layout();
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&middle_nodes[2]);

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &left_node, true);

    let normal_node_port = add_port_on_side(&right_nodes[1], PortSide::West);
    let dummy_node_port = add_port_on_side(&middle_nodes[1], PortSide::East);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    let origin_port = middle_nodes[2]
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports().get(0).cloned())
        .expect("origin port");
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(origin_port)));
    }

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &right_nodes[0], false);

    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 0);
}

#[test]
fn edges_in_both_directions() {
    init_layout();
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&middle_nodes[2]);

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &left_nodes[1], true);

    let normal_node_port = add_port_on_side(&right_nodes[1], PortSide::West);
    let dummy_node_port = add_port_on_side(&middle_nodes[1], PortSide::East);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    let origin_port = middle_nodes[2]
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports().get(0).cloned())
        .expect("origin port");
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(origin_port)));
    }

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &left_nodes[0], true);

    let normal_node_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let dummy_node_port = add_port_on_side(&middle_nodes[0], PortSide::East);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    let origin_port = middle_nodes[2]
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports().get(1).cloned())
        .expect("origin port");
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(origin_port)));
    }

    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 1);
    assert_eq!(lower_upper, 1);
}

#[test]
fn multiple_edges_in_both_directions_ns_node() {
    init_layout();
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    set_fixed_order_constraint(&middle_nodes[2]);

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[1], &left_nodes[1], true);

    let normal_node_port = add_port_on_side(&right_nodes[1], PortSide::West);
    let dummy_node_port = add_port_on_side(&middle_nodes[1], PortSide::East);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    let origin_port = middle_nodes[2]
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports().get(0).cloned())
        .expect("origin port");
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(origin_port)));
    }

    add_north_south_edge(PortSide::North, &middle_nodes[2], &middle_nodes[0], &right_nodes[0], false);

    let normal_node_port = add_port_on_side(&right_nodes[0], PortSide::East);
    let dummy_node_port = add_port_on_side(&middle_nodes[0], PortSide::West);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    add_edge_between_ports(&dummy_node_port, &normal_node_port);
    let origin_port = middle_nodes[2]
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.ports().get(1).cloned())
        .expect("origin port");
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LPort(origin_port)));
    }

    let layer = layer_nodes(&graph, 1);
    let (upper_lower, lower_upper) = count_crossings(&layer, 0, 1);
    assert_eq!(upper_lower, 2);
    assert_eq!(lower_upper, 2);
}
