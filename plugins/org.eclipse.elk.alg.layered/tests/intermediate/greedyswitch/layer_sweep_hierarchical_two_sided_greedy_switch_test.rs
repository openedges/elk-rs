use std::sync::{Arc, OnceLock};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredMetaDataProvider, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::{
    CrossMinType, LayerSweepCrossingMinimizer,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, EnumSet, Random};

fn init_layered_options() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
    });
}

fn new_graph() -> LGraphRef {
    init_layered_options();
    let graph = LGraph::new();
    {
        let mut graph_guard = graph.lock();
        graph_guard.set_property(InternalProperties::RANDOM, Some(Random::new(99)));
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
    }
    graph
}

fn make_layer(graph: &LGraphRef) -> LayerRef {
    let layer = Layer::new(graph);
    {
        let mut graph_guard = graph.lock();
        graph_guard.layers_mut().push(layer.clone());
    }
    layer
}

fn make_layers(graph: &LGraphRef, count: usize) -> Vec<LayerRef> {
    (0..count).map(|_| make_layer(graph)).collect()
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port_on_side(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock();
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    {
        let mut node_guard = node.lock();
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

fn add_edge_between_ports(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn add_east_west_edge(source: &LNodeRef, target: &LNodeRef) {
    let source_port = add_port_on_side(source, PortSide::East);
    let target_port = add_port_on_side(target, PortSide::West);
    add_edge_between_ports(&source_port, &target_port);
}

fn add_east_west_edge_from_port(source: &LPortRef, target: &LNodeRef) {
    let target_port = add_port_on_side(target, PortSide::West);
    add_edge_between_ports(source, &target_port);
}

fn add_east_west_edge_from_node(source: &LNodeRef, target: &LPortRef) {
    let source_port = add_port_on_side(source, PortSide::East);
    add_edge_between_ports(&source_port, target);
}

fn nested_graph(node: &LNodeRef) -> LGraphRef {
    if let Some(existing) = node.lock().nested_graph() {
        return existing;
    }
    let nested = new_graph();
    {
        let mut nested_guard = nested.lock();
        nested_guard.set_parent_node(Some(node.clone()));
    }
    {
        let mut node_guard = node.lock();
        node_guard.set_nested_graph(Some(nested.clone()));
    }
    nested
}

fn add_external_port_dummy_node_to_layer(layer: &LayerRef, port: &LPortRef) -> LNodeRef {
    let graph = layer
        .lock().graph()
        .expect("layer graph");
    let node = add_node(&graph, layer);
    let port_side = port
        .lock().side();
    {
        let mut node_guard = node.lock();
        node_guard.set_node_type(NodeType::ExternalPort);
        node_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(port.clone())),
        );
        node_guard.set_property(InternalProperties::EXT_PORT_SIDE, Some(port_side));
    }
    {
        let mut port_guard = port.lock();
        port_guard.set_property(InternalProperties::PORT_DUMMY, Some(node.clone()));
        port_guard.set_property(InternalProperties::INSIDE_CONNECTIONS, Some(true));
    }
    {
        let mut graph_guard = graph.lock();
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
        .lock().side();
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

fn clone_layer_nodes(layer: &LayerRef) -> Vec<LNodeRef> {
    layer.lock().nodes().clone()
}

fn same_node_order(actual: &[LNodeRef], expected: &[LNodeRef]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(left, right)| Arc::ptr_eq(left, right))
}

fn run_two_sided_crossmin(graph: &LGraphRef) {
    let mut minimizer = LayerSweepCrossingMinimizer::new(CrossMinType::TwoSidedGreedySwitch);
    let mut monitor = BasicProgressMonitor::new();
    minimizer.process(&mut graph.lock(), &mut monitor);
}

fn node_index(nodes: &[LNodeRef], needle: &LNodeRef) -> usize {
    nodes
        .iter()
        .position(|node| Arc::ptr_eq(node, needle))
        .expect("node should exist in layer")
}

fn set_fixed_order_constraint(node: &LNodeRef) {
    {
        let mut node_guard = node.lock();
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedOrder),
        );
        if let Some(graph) = node_guard.graph() {
            {
                let mut graph_guard = graph.lock();
                let mut props = graph_guard
                    .get_property(InternalProperties::GRAPH_PROPERTIES)
                    .unwrap_or_else(EnumSet::none_of);
                props.insert(GraphProperties::NonFreePorts);
                graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
            }
        }
    }
}

#[test]
fn two_sided_greedy_switch_runs_on_simple_cross_graph() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);
    let n1 = add_node(&graph, &left);
    let n2 = add_node(&graph, &left);
    let n3 = add_node(&graph, &right);
    let n4 = add_node(&graph, &right);
    add_east_west_edge(&n1, &n4);
    add_east_west_edge(&n2, &n3);

    run_two_sided_crossmin(&graph);
}

#[test]
fn two_sided_greedy_switch_runs_with_nested_graph() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);
    let outer = add_node(&graph, &left);
    let right_top = add_node(&graph, &right);
    let right_bottom = add_node(&graph, &right);
    add_east_west_edge(&outer, &right_top);
    add_east_west_edge(&outer, &right_bottom);

    let nested = new_graph();
    let nested_left = make_layer(&nested);
    let nested_right = make_layer(&nested);
    let inner1 = add_node(&nested, &nested_left);
    let inner2 = add_node(&nested, &nested_left);
    let inner3 = add_node(&nested, &nested_right);
    let inner4 = add_node(&nested, &nested_right);
    add_east_west_edge(&inner1, &inner4);
    add_east_west_edge(&inner2, &inner3);

    {
        let mut outer_guard = outer.lock();
        outer_guard.set_nested_graph(Some(nested));
    }

    run_two_sided_crossmin(&graph);

    let graph_guard = graph.lock();    assert_eq!(graph_guard.layers().len(), 2);
}

#[test]
fn two_sided_greedy_switch_removes_cross_in_single_nested_graph() {
    let graph = new_graph();
    let outer_layer = make_layer(&graph);
    let outer = add_node(&graph, &outer_layer);

    let nested = new_graph();
    let left = make_layer(&nested);
    let right = make_layer(&nested);
    let l0 = add_node(&nested, &left);
    let l1 = add_node(&nested, &left);
    let r0 = add_node(&nested, &right);
    let r1 = add_node(&nested, &right);
    add_east_west_edge(&l0, &r1);
    add_east_west_edge(&l1, &r0);

    {
        let mut outer_guard = outer.lock();
        outer_guard.set_nested_graph(Some(nested.clone()));
    }

    run_two_sided_crossmin(&graph);

    let left_nodes = nested.lock().layers()[0]
        .lock()
        
        .nodes()
        .clone();
    let right_nodes = nested.lock().layers()[1]
        .lock()
        
        .nodes()
        .clone();

    let left_delta = node_index(&left_nodes, &l0) as isize - node_index(&left_nodes, &l1) as isize;
    let right_delta =
        node_index(&right_nodes, &r1) as isize - node_index(&right_nodes, &r0) as isize;
    assert!(
        left_delta.signum() == right_delta.signum(),
        "expected nested crossing (l0->r1, l1->r0) to be removed"
    );
}

#[test]
fn two_sided_greedy_switch_forward_sweep_case_reorders_inner_layer() {
    let graph = new_graph();
    let outer_layer = make_layer(&graph);
    let outer = add_node(&graph, &outer_layer);

    let nested = new_graph();
    let left_layer = make_layer(&nested);
    let right_layer = make_layer(&nested);
    let left = add_node(&nested, &left_layer);
    set_fixed_order_constraint(&left);
    let r0 = add_node(&nested, &right_layer);
    let r1 = add_node(&nested, &right_layer);
    add_east_west_edge(&left, &r1);
    add_east_west_edge(&left, &r0);

    {
        let mut outer_guard = outer.lock();
        outer_guard.set_nested_graph(Some(nested.clone()));
    }

    let before_right = right_layer
        .lock()
        
        .nodes()
        .clone();
    run_two_sided_crossmin(&graph);
    let after_right = right_layer
        .lock()
        
        .nodes()
        .clone();

    assert_eq!(after_right.len(), 2);
    assert!(
        Arc::ptr_eq(&after_right[0], &before_right[1])
            && Arc::ptr_eq(&after_right[1], &before_right[0]),
        "expected nested right layer order to switch in forward-sweep case"
    );
}

#[test]
fn given_cross_in_first_level_compound_node_using_two_sided_greedy_switch_leaves_crossing() {
    let graph = new_graph();
    let left_layer = make_layer(&graph);
    let right_layer = make_layer(&graph);
    let left_outer_node = add_node(&graph, &left_layer);
    let right_nodes = [
        add_node(&graph, &right_layer),
        add_node(&graph, &right_layer),
    ];
    let left_outer_ports = add_ports_on_side(&left_outer_node, 2, PortSide::East);
    add_east_west_edge_from_port(&left_outer_ports[0], &right_nodes[0]);
    add_east_west_edge_from_port(&left_outer_ports[1], &right_nodes[1]);

    let left_inner_graph = nested_graph(&left_outer_node);
    let left_inner_left_layer = make_layer(&left_inner_graph);
    let left_inner_right_layer = make_layer(&left_inner_graph);
    let left_inner_dummy_layer = make_layer(&left_inner_graph);
    let left_inner_nodes_left = [
        add_node(&left_inner_graph, &left_inner_left_layer),
        add_node(&left_inner_graph, &left_inner_left_layer),
    ];
    let left_inner_nodes_right = [
        add_node(&left_inner_graph, &left_inner_right_layer),
        add_node(&left_inner_graph, &left_inner_right_layer),
    ];
    let left_inner_dummy_nodes =
        add_external_port_dummies_to_layer(&left_inner_dummy_layer, &left_outer_ports);
    add_east_west_edge(&left_inner_nodes_right[0], &left_inner_dummy_nodes[0]);
    add_east_west_edge(&left_inner_nodes_right[1], &left_inner_dummy_nodes[1]);
    add_east_west_edge(&left_inner_nodes_left[0], &left_inner_nodes_right[1]);
    add_east_west_edge(&left_inner_nodes_left[1], &left_inner_nodes_right[0]);
    {
        let mut graph_guard = graph.lock();
        graph_guard.set_property(
            LayeredOptions::CROSSING_MINIMIZATION_HIERARCHICAL_SWEEPINESS,
            Some(-1.0),
        );
    }

    let expected_same_order = clone_layer_nodes(&right_layer);
    run_two_sided_crossmin(&graph);

    assert!(
        same_node_order(&clone_layer_nodes(&right_layer), &expected_same_order),
        "expected first-level right layer order to remain unchanged"
    );
}

#[test]
fn only_two_sided_greedy_switch_returns_no_change() {
    let graph = new_graph();
    let left_node = add_node(&graph, &make_layer(&graph));
    let right_node = add_node(&graph, &make_layer(&graph));
    let left_ports = add_ports_on_side(&left_node, 2, PortSide::East);
    add_east_west_edge_from_port(&left_ports[0], &right_node);
    add_east_west_edge_from_port(&left_ports[1], &right_node);
    set_fixed_order_constraint(&right_node);

    let inner_graph = nested_graph(&left_node);
    let inner_layers = make_layers(&inner_graph, 2);
    let inner_left_node = add_node(&inner_graph, &inner_layers[0]);
    set_fixed_order_constraint(&inner_left_node);
    let dummies = add_external_port_dummies_to_layer(&inner_layers[1], &left_ports);
    add_east_west_edge(&inner_left_node, &dummies[0]);
    add_east_west_edge(&inner_left_node, &dummies[1]);

    let expected_order = clone_layer_nodes(&inner_layers[1]);
    run_two_sided_crossmin(&graph);

    assert!(
        same_node_order(&clone_layer_nodes(&inner_layers[1]), &expected_order),
        "expected dummy order to remain unchanged with two-sided greedy switch"
    );
}

#[test]
fn crossing_before_last_layer_causes_crossing_outside_two_sided_prevents() {
    let graph = new_graph();
    let left_node = add_node(&graph, &make_layer(&graph));
    let right_node = add_node(&graph, &make_layer(&graph));
    let left_ports = add_ports_on_side(&left_node, 2, PortSide::East);
    add_east_west_edge_from_port(&left_ports[1], &right_node);
    add_east_west_edge_from_port(&left_ports[0], &right_node);
    set_fixed_order_constraint(&right_node);

    let inner_graph = nested_graph(&left_node);
    let inner_layers = make_layers(&inner_graph, 2);
    let inner_left_node = add_node(&inner_graph, &inner_layers[0]);
    set_fixed_order_constraint(&inner_left_node);
    let dummies = add_external_port_dummies_to_layer(&inner_layers[1], &left_ports);
    add_east_west_edge(&inner_left_node, &dummies[1]);
    add_east_west_edge(&inner_left_node, &dummies[0]);

    let expected_order = clone_layer_nodes(&inner_layers[1]);
    run_two_sided_crossmin(&graph);

    assert!(
        same_node_order(&clone_layer_nodes(&inner_layers[1]), &expected_order),
        "expected dummy order to remain unchanged to prevent outside crossing"
    );
}

#[test]
fn crossing_before_first_layer_causes_crossing_outside_two_sided_prevents() {
    let graph = new_graph();
    {
        let mut graph_guard = graph.lock();
        let mut random = Random::new(0);
        random.set_mock_next_boolean(false);
        random.set_mock_double_sequence(0.0, 0.0001);
        graph_guard.set_property(InternalProperties::RANDOM, Some(random));
    }
    let left_node = add_node(&graph, &make_layer(&graph));
    let right_node = add_node(&graph, &make_layer(&graph));
    let right_ports = add_ports_on_side(&right_node, 2, PortSide::West);
    add_east_west_edge_from_node(&left_node, &right_ports[1]);
    add_east_west_edge_from_node(&left_node, &right_ports[0]);
    set_fixed_order_constraint(&left_node);

    let inner_graph = nested_graph(&right_node);
    let inner_layers = make_layers(&inner_graph, 2);
    let inner_right_node = add_node(&inner_graph, &inner_layers[1]);
    set_fixed_order_constraint(&inner_right_node);
    let dummies = add_external_port_dummies_to_layer(&inner_layers[0], &right_ports);
    add_east_west_edge(&dummies[0], &inner_right_node);
    add_east_west_edge(&dummies[1], &inner_right_node);

    let expected_order = clone_layer_nodes(&inner_layers[0]);
    run_two_sided_crossmin(&graph);

    assert!(
        same_node_order(&clone_layer_nodes(&inner_layers[0]), &expected_order),
        "expected dummy order to remain unchanged to prevent outside crossing"
    );
}
