use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredMetaDataProvider, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::graph_info_holder::GraphInfoHolder;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::CrossMinType;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

fn init_layered_options() {
    let service = LayoutMetaDataService::get_instance();
    service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

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

fn make_layers(count: usize, graph: &LGraphRef) -> Vec<LayerRef> {
    (0..count).map(|_| make_layer(graph)).collect()
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
        connect(&normal_node_port, &dummy_node_port);
    } else {
        connect(&dummy_node_port, &normal_node_port);
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
}

fn set_on_all_graphs<T: Clone + Send + Sync + 'static>(
    graph: &LGraphRef,
    property: &Property<T>,
    value: T,
) {
    let nested_graphs: Vec<LGraphRef> = if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(property, Some(value.clone()));
        graph_guard
            .layers()
            .iter()
            .filter_map(|layer| layer.lock().ok().map(|layer_guard| layer_guard.nodes().clone()))
            .flatten()
            .filter_map(|node| node.lock().ok().and_then(|node_guard| node_guard.nested_graph()))
            .collect()
    } else {
        Vec::new()
    };

    for nested in nested_graphs {
        set_on_all_graphs(&nested, property, value.clone());
    }
}

#[test]
fn north_south_ports_change_decision() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let layers = make_layers(3, &graph);
    let left_outer = add_node(&graph, &layers[0]);
    let middle_outer = add_node(&graph, &layers[1]);
    let right_outer = add_node(&graph, &layers[2]);

    let left_port_middle = add_port_on_side(&middle_outer, PortSide::West);
    let right_port_middle = add_port_on_side(&middle_outer, PortSide::East);

    east_west_edge_from_to(&middle_outer, &right_outer);
    east_west_edge_from_port(&right_port_middle, &right_outer);
    east_west_edge_to_port(&left_outer, &left_port_middle);
    east_west_edge_from_to(&left_outer, &middle_outer);

    let nested = nested_graph(&middle_outer);
    let inner_layers = make_layers(5, &nested);
    let left_dummy = add_external_port_dummy_node_to_layer(&inner_layers[0], &left_port_middle);
    let first_layer = add_nodes(&nested, &inner_layers[1], 3);
    let second_layer = add_nodes(&nested, &inner_layers[2], 3);
    let third_layer = add_nodes(&nested, &inner_layers[3], 2);
    let right_dummy = add_external_port_dummy_node_to_layer(&inner_layers[4], &right_port_middle);

    east_west_edge_from_to(&left_dummy, &first_layer[2]);
    east_west_edge_from_to(&first_layer[2], &second_layer[2]);
    east_west_edge_from_to(&second_layer[2], &third_layer[1]);
    east_west_edge_from_to(&third_layer[1], &right_dummy);

    add_north_south_edge(
        PortSide::North,
        &first_layer[1],
        &first_layer[0],
        &second_layer[0],
        false,
    );
    add_north_south_edge(
        PortSide::South,
        &second_layer[0],
        &second_layer[1],
        &third_layer[0],
        false,
    );

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.graph_element().id = 0;
    }
    if let Ok(mut graph_guard) = nested.lock() {
        graph_guard.graph_element().id = 1;
    }
    set_on_all_graphs(
        &graph,
        LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
        0.1,
    );

    let nested_info = GraphInfoHolder::new(nested.clone(), CrossMinType::Barycenter);
    assert!(nested_info.dont_sweep_into());
}

#[test]
fn all_hierarchical_but_minus_one_still_returns_bottom_up() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let left_node = add_node(&graph, &make_layer(&graph));
    let middle_node = add_node(&graph, &make_layer(&graph));
    let right_node = add_node(&graph, &make_layer(&graph));

    let middle_ports_right = add_ports_on_side(&middle_node, 2, PortSide::East);
    let middle_ports_left = add_ports_on_side(&middle_node, 2, PortSide::West);
    east_west_edge_to_port(&left_node, &middle_ports_left[1]);
    east_west_edge_to_port(&left_node, &middle_ports_left[0]);
    east_west_edge_from_port(&middle_ports_right[0], &right_node);
    east_west_edge_from_port(&middle_ports_right[1], &right_node);

    let inner = nested_graph(&middle_node);
    let inner_layers = make_layers(3, &inner);
    let left_inner_dummy_nodes = add_external_port_dummies_to_layer(&inner_layers[2], &middle_ports_right);
    let inner_node = add_node(&inner, &inner_layers[1]);
    let right_inner_dummy_nodes = add_external_port_dummies_to_layer(&inner_layers[0], &middle_ports_left);
    east_west_edge_from_to(&left_inner_dummy_nodes[0], &inner_node);
    east_west_edge_from_to(&left_inner_dummy_nodes[1], &inner_node);
    east_west_edge_from_to(&inner_node, &right_inner_dummy_nodes[0]);
    east_west_edge_from_to(&inner_node, &right_inner_dummy_nodes[1]);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.graph_element().id = 0;
    }
    if let Ok(mut graph_guard) = inner.lock() {
        graph_guard.graph_element().id = 1;
    }

    set_on_all_graphs(
        &graph,
        LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
        -1.0,
    );

    let nested_info = GraphInfoHolder::new(inner.clone(), CrossMinType::Barycenter);
    assert!(nested_info.dont_sweep_into());
}

#[test]
fn port_with_no_edges_does_not_count() {
    init_layered_options();
    let graph = LGraph::new();
    set_up_graph(&graph);

    let middle_node = add_node(&graph, &make_layer(&graph));
    let right_node = add_node(&graph, &make_layer(&graph));
    let middle_ports_right = add_ports_on_side(&middle_node, 2, PortSide::East);
    east_west_edge_from_port(&middle_ports_right[0], &right_node);
    east_west_edge_from_port(&middle_ports_right[1], &right_node);

    let inner = nested_graph(&middle_node);
    let inner_layers = make_layers(2, &inner);
    let inner_node = add_node(&inner, &inner_layers[0]);
    let right_inner_dummy_nodes = add_external_port_dummies_to_layer(&inner_layers[1], &middle_ports_right);
    east_west_edge_from_to(&inner_node, &right_inner_dummy_nodes[0]);
    east_west_edge_from_to(&inner_node, &right_inner_dummy_nodes[1]);
    add_port_on_side(&inner_node, PortSide::West);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.graph_element().id = 0;
    }
    if let Ok(mut graph_guard) = inner.lock() {
        graph_guard.graph_element().id = 1;
    }

    set_on_all_graphs(
        &graph,
        LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
        -0.1,
    );

    let nested_info = GraphInfoHolder::new(inner.clone(), CrossMinType::Barycenter);
    assert!(nested_info.dont_sweep_into());
}
