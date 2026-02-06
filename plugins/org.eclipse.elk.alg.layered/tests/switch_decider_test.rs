#![allow(dead_code)]

use std::sync::OnceLock;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::greedyswitch::{
    CrossingCountSide, CrossingMatrixFiller, SwitchDecider,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::greedyswitch::switch_decider::ParentCrossingContext;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::CrossMinType;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

fn init_reflect() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        LayoutMetaDataService::get_instance();
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

fn add_ports_on_side(node: &LNodeRef, count: usize, side: PortSide) -> Vec<LPortRef> {
    (0..count).map(|_| add_port_on_side(node, side)).collect()
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

fn nested_graph(node: &LNodeRef) -> LGraphRef {
    if let Some(nested_graph) = node.lock().ok().and_then(|node_guard| node_guard.nested_graph()) {
        return nested_graph;
    }

    let nested_graph = LGraph::new();
    if let Ok(mut graph_guard) = nested_graph.lock() {
        graph_guard.set_parent_node(Some(node.clone()));
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
        graph_guard.set_property(InternalProperties::RANDOM, Some(Random::new(0)));
    }
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_nested_graph(Some(nested_graph.clone()));
    }
    nested_graph
}

fn add_external_port_dummy_node_to_layer(layer: &LayerRef, port: &LPortRef) -> LNodeRef {
    let graph = layer.lock().ok().and_then(|layer_guard| layer_guard.graph()).expect("layer graph");
    let node = add_node_to_layer(&graph, layer);
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

fn assign_ids(root: &LGraphRef) {
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

fn count_ports(node_order: &[Vec<LNodeRef>]) -> usize {
    let mut count = 0usize;
    for layer in node_order {
        for node in layer {
            if let Ok(node_guard) = node.lock() {
                count += node_guard.ports().len();
            }
        }
    }
    count
}

fn get_decider(
    graph: &LGraphRef,
    greedy_type: CrossMinType,
    free_layer_index: usize,
    direction: CrossingCountSide,
) -> (SwitchDecider, Vec<Vec<LNodeRef>>) {
    assign_ids(graph);
    let node_order = graph.lock().expect("graph lock").to_node_array();
    let n_ports = count_ports(&node_order);
    let filler = CrossingMatrixFiller::new(greedy_type, &node_order, free_layer_index, direction);
    let decider = SwitchDecider::new(
        node_order
            .get(free_layer_index)
            .map(|layer| layer.as_slice())
            .unwrap_or(&[]),
        filler,
        &vec![0; n_ports],
        None,
        false,
    );
    (decider, node_order)
}

fn get_decider_with_parent(
    graph: &LGraphRef,
    parent_graph: &LGraphRef,
    parent_node: &LNodeRef,
    greedy_type: CrossMinType,
    free_layer_index: usize,
    direction: CrossingCountSide,
) -> (SwitchDecider, Vec<Vec<LNodeRef>>) {
    assign_ids(graph);
    assign_ids(parent_graph);
    let node_order = graph.lock().expect("graph lock").to_node_array();
    let n_ports = count_ports(&node_order);
    let parent_node_order = parent_graph.lock().expect("graph lock").to_node_array();
    let parent_ports = count_ports(&parent_node_order);
    let parent_layer_index = parent_node
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| layer.lock().ok().and_then(|layer_guard| layer_guard.index()))
        .unwrap_or(0);
    let right_most_layer = free_layer_index + 1 == node_order.len();
    let parent_context = ParentCrossingContext::new(
        parent_node_order,
        vec![0; parent_ports],
        parent_layer_index,
        right_most_layer,
    );
    let filler = CrossingMatrixFiller::new(greedy_type, &node_order, free_layer_index, direction);
    let decider = SwitchDecider::new(
        node_order
            .get(free_layer_index)
            .map(|layer| layer.as_slice())
            .unwrap_or(&[]),
        filler,
        &vec![0; n_ports],
        Some(parent_context),
        true,
    );
    (decider, node_order)
}

fn switch_nodes(node_order: &mut [Vec<LNodeRef>], free_layer_index: usize, upper: usize, lower: usize) {
    if let Some(layer) = node_order.get_mut(free_layer_index) {
        layer.swap(upper, lower);
    }
}

fn node_at(
    node_order: &[Vec<LNodeRef>],
    layer_index: usize,
    node_index: usize,
) -> &LNodeRef {
    &node_order[layer_index][node_index]
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

fn graph_one_node() -> LGraphRef {
    let graph = new_graph();
    let layer = make_layer(&graph);
    add_node_to_layer(&graph, &layer);
    graph
}

fn graph_multiple_edges_between_same_nodes() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    graph
}

fn graph_cross_with_many_self_loops() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 2);

    let top_left_port = add_port_on_side(&left_nodes[0], PortSide::East);
    let bottom_left_port = add_port_on_side(&left_nodes[1], PortSide::East);

    for layer in graph.lock().expect("graph lock").layers().clone() {
        let nodes = layer.lock().expect("layer lock").nodes().clone();
        for node in nodes {
            self_loop_on(&node, PortSide::East);
            self_loop_on(&node, PortSide::East);
            self_loop_on(&node, PortSide::East);
            self_loop_on(&node, PortSide::West);
            self_loop_on(&node, PortSide::West);
            self_loop_on(&node, PortSide::West);
        }
    }

    let top_right_port = add_port_on_side(&right_nodes[0], PortSide::West);
    let bottom_right_port = add_port_on_side(&right_nodes[1], PortSide::West);
    add_edge_between_ports(&top_left_port, &bottom_right_port);
    add_edge_between_ports(&bottom_left_port, &top_right_port);
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
    let right_upper = add_port_on_side(&right_nodes[0], PortSide::West);
    let right_middle = add_port_on_side(&right_nodes[1], PortSide::West);

    add_edge_between_ports(&middle_upper_east, &right_upper);
    add_edge_between_ports(&middle_upper_east, &right_middle);
    add_edge_between_ports(&middle_upper_east, &right_middle);
    east_west_edge_from_port(&middle_lower_east, &right_nodes[2]);
    east_west_edge_from_port(&left_middle_port, &middle_nodes[0]);
    east_west_edge_from_node(&middle_nodes[1], &right_upper);
    east_west_edge_from_port(&left_middle_port, &middle_nodes[1]);
    east_west_edge_from_to(&left_nodes[2], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);
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
    graph
}

fn graph_switch_only_east_one_sided() -> LGraphRef {
    let graph = new_graph();
    let layers = make_layers(&graph, 3);
    let left_nodes = add_nodes_to_layer(&graph, &layers[0], 2);
    let middle_nodes = add_nodes_to_layer(&graph, &layers[1], 2);
    let right_nodes = add_nodes_to_layer(&graph, &layers[2], 2);

    east_west_edge_from_to(&left_nodes[0], &middle_nodes[0]);
    east_west_edge_from_to(&left_nodes[1], &middle_nodes[1]);
    east_west_edge_from_to(&middle_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[0]);
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
    graph
}

fn graph_three_layer_north_south_crossing() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 3);
    let right_node = add_node_to_layer(&graph, &right_layer);

    set_fixed_order_constraint(&middle_nodes[0]);
    set_fixed_order_constraint(&right_node);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[2], &right_node, false);
    add_north_south_edge(PortSide::South, &middle_nodes[0], &middle_nodes[1], &right_node, false);
    east_west_edge_from_to(&left_node, &middle_nodes[0]);
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

fn graph_switched_problem() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 4);

    east_west_edge_from_to(&left_nodes[1], &right_nodes[2]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[3]);

    east_west_edge_from_to(&left_nodes[0], &right_nodes[0]);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[2]);
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

fn graph_layout_unit_prevents_switch_with_node_with_northern_edges() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 3);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    add_north_south_edge(PortSide::North, &left_nodes[1], &left_nodes[0], &right_nodes[0], false);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[2]);
    east_west_edge_from_to(&left_nodes[2], &right_nodes[1]);
    graph
}

fn graph_layout_unit_prevents_switch_with_node_with_southern_edges() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_nodes = add_nodes_to_layer(&graph, &left_layer, 4);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    add_north_south_edge(PortSide::South, &left_nodes[1], &left_nodes[2], &right_nodes[2], false);
    graph
}

fn graph_layout_unit_does_not_prevent_switch_with_long_edge_dummy() -> LGraphRef {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let middle_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_node = add_node_to_layer(&graph, &left_layer);
    let middle_nodes = add_nodes_to_layer(&graph, &middle_layer, 4);
    let right_nodes = add_nodes_to_layer(&graph, &right_layer, 3);

    set_as_long_edge_dummy(&middle_nodes[0]);
    east_west_edge_from_to(&left_node, &middle_nodes[0]);
    east_west_edge_from_to(&middle_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[0]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[0]);
    add_north_south_edge(PortSide::South, &middle_nodes[1], &middle_nodes[2], &right_nodes[2], false);
    graph
}

fn for_each_greedy_type<F: FnMut(CrossMinType)>(mut f: F) {
    for greedy in [
        CrossMinType::OneSidedGreedySwitch,
        CrossMinType::TwoSidedGreedySwitch,
    ] {
        f(greedy);
    }
}

#[test]
fn cross_formed() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_formed();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));
    });
}

#[test]
fn one_node() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_one_node();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::West);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 0)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 0)
        ));
    });
}

#[test]
fn multiple_edges_between_same_nodes() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_multiple_edges_between_same_nodes();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));
    });
}

#[test]
fn self_loops() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_with_many_self_loops();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));
    });
}

#[test]
fn north_south_port_crossing() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_three_layer_north_south_crossing();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        let result = decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 1),
            node_at(&node_order, 1, 2),
        );
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert!(result);
        } else {
            assert!(!result);
        }
    });
}

#[test]
fn more_complex() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_more_complex_three_layer();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 2, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 2, 0),
            node_at(&node_order, 2, 1)
        ));
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 2, 1),
            node_at(&node_order, 2, 2)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::East);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));

        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 1),
            node_at(&node_order, 0, 2)
        ));
    });
}

#[test]
fn switch_only_true_for_one_sided() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_switch_only_one_sided();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        let result = decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1),
        );
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert!(result);
        } else {
            assert!(!result);
        }
    });
}

#[test]
fn switch_only_true_for_one_sided_eastern_side() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_switch_only_east_one_sided();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::East);
        let result = decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1),
        );
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert!(result);
        } else {
            assert!(!result);
        }
    });
}

#[test]
fn constraints_prevent_switch() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_formed_with_constraints_in_second_layer();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
    });
}

#[test]
fn in_layer_unit_constraints_prevent_switch() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_where_layout_unit_prevents_switch();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::West);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 1),
            node_at(&node_order, 0, 2)
        ));
    });
}

#[test]
fn switch_and_recount() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_cross_formed();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));

        let (mut decider, mut node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));

        let upper = node_at(&node_order, 0, 0).clone();
        let lower = node_at(&node_order, 0, 1).clone();
        switch_nodes(&mut node_order, 0, 0, 1);
        decider.notify_of_switch(&upper, &lower);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));
    });
}

#[test]
fn switch_and_recount_counter_bug() {
    for_each_greedy_type(|greedy_type| {
        let graph = new_graph();
        let left_layer = make_layer(&graph);
        let right_layer = make_layer(&graph);
        let left_nodes = add_nodes_to_layer(&graph, &left_layer, 2);
        let right_nodes = add_nodes_to_layer(&graph, &right_layer, 4);
        let left_top_port = add_port_on_side(&left_nodes[0], PortSide::East);
        let left_lower_port = add_port_on_side(&left_nodes[1], PortSide::East);
        let right_top_port = add_port_on_side(&right_nodes[0], PortSide::West);

        add_edge_between_ports(&left_lower_port, &right_top_port);
        east_west_edge_from_port(&left_lower_port, &right_nodes[2]);

        add_edge_between_ports(&left_top_port, &right_top_port);
        east_west_edge_from_port(&left_top_port, &right_nodes[1]);
        east_west_edge_from_port(&left_top_port, &right_nodes[3]);

        let (mut decider, mut node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 1),
            node_at(&node_order, 1, 2)
        ));
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 2),
            node_at(&node_order, 1, 3)
        ));

        let upper = node_at(&node_order, 1, 0).clone();
        let lower = node_at(&node_order, 1, 1).clone();
        decider.notify_of_switch(&upper, &lower);
        switch_nodes(&mut node_order, 1, 0, 1);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 1),
            node_at(&node_order, 1, 2)
        ));
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 2),
            node_at(&node_order, 1, 3)
        ));

        let upper = node_at(&node_order, 1, 2).clone();
        let lower = node_at(&node_order, 1, 3).clone();
        decider.notify_of_switch(&upper, &lower);
        switch_nodes(&mut node_order, 1, 2, 3);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 1),
            node_at(&node_order, 1, 2)
        ));
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 2),
            node_at(&node_order, 1, 3)
        ));

        let upper = node_at(&node_order, 1, 1).clone();
        let lower = node_at(&node_order, 1, 2).clone();
        decider.notify_of_switch(&upper, &lower);
        switch_nodes(&mut node_order, 1, 1, 2);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 1),
            node_at(&node_order, 1, 2)
        ));
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 2),
            node_at(&node_order, 1, 3)
        ));
    });
}

#[test]
fn switch_and_recount_reduced_counter_bug() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_switched_problem();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        let len = node_order[1].len();
        for i in 0..len - 1 {
            assert!(
                !decider.does_switch_reduce_crossings(
                    node_at(&node_order, 1, i),
                    node_at(&node_order, 1, i + 1)
                ),
                "attempted switch {} with {}",
                i,
                i + 1
            );
        }
    });
}

#[test]
fn should_switch_with_long_edge_dummies() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_northern_north_south_dummy_edge_crossing();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 1),
            node_at(&node_order, 1, 2)
        ));
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert!(decider.does_switch_reduce_crossings(
                node_at(&node_order, 1, 0),
                node_at(&node_order, 1, 1)
            ));
        }

        let graph = graph_southern_north_south_dummy_edge_crossing();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::West);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
        if greedy_type == CrossMinType::OneSidedGreedySwitch {
            assert!(decider.does_switch_reduce_crossings(
                node_at(&node_order, 1, 1),
                node_at(&node_order, 1, 2)
            ));
        }
    });
}

#[test]
fn layout_unit_constraint_prevents_switch_with_node_with_northern_ports() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_layout_unit_prevents_switch_with_node_with_northern_edges();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 1),
            node_at(&node_order, 0, 2)
        ));
    });
}

#[test]
fn layout_unit_constraint_prevents_switch_with_node_with_southern_ports() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_layout_unit_prevents_switch_with_node_with_southern_edges();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 0, CrossingCountSide::East);
        assert!(!decider.does_switch_reduce_crossings(
            node_at(&node_order, 0, 0),
            node_at(&node_order, 0, 1)
        ));
    });
}

#[test]
fn layout_unit_constraint_does_not_prevent_switch_with_long_edge_dummy() {
    for_each_greedy_type(|greedy_type| {
        let graph = graph_layout_unit_does_not_prevent_switch_with_long_edge_dummy();
        let (mut decider, node_order) = get_decider(&graph, greedy_type, 1, CrossingCountSide::East);
        assert!(decider.does_switch_reduce_crossings(
            node_at(&node_order, 1, 0),
            node_at(&node_order, 1, 1)
        ));
    });
}

#[test]
fn switching_dummy_nodes_notifies_port_switch() {
    for_each_greedy_type(|greedy_type| {
        let graph = new_graph();
        let left_node = add_node_to_layer(&graph, &make_layer(&graph));
        let right_nodes = add_nodes_to_layer(&graph, &make_layer(&graph), 2);
        let left_ports = add_ports_on_side(&left_node, 2, PortSide::East);
        let nested_graph = nested_graph(&left_node);
        let nested_layer = make_layer(&nested_graph);
        let dummies = add_external_port_dummies_to_layer(&nested_layer, &left_ports);
        east_west_edge_from_port(&left_ports[0], &right_nodes[1]);
        east_west_edge_from_port(&left_ports[1], &right_nodes[0]);

        if greedy_type == CrossMinType::TwoSidedGreedySwitch {
            let (mut decider, _node_order) = get_decider_with_parent(
                &nested_graph,
                &graph,
                &left_node,
                greedy_type,
                0,
                CrossingCountSide::East,
            );
            assert!(decider.does_switch_reduce_crossings(&dummies[0], &dummies[1]));
            decider.notify_of_switch(&dummies[0], &dummies[1]);
            assert!(!decider.does_switch_reduce_crossings(&dummies[0], &dummies[1]));
        }
    });
}
