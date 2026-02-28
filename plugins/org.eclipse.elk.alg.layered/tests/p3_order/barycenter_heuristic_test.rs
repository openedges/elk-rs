use std::sync::{Arc, OnceLock};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredMetaDataProvider, LayeredOptions, Origin,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::barycenter_heuristic::BarycenterHeuristic;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::i_initializable::{
    init, IInitializable,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::node_relative_port_distributor::NodeRelativePortDistributor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{EnumSet, Random};

#[test]
fn minimize_crossings_removes_crossing_in_simple_cross() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);

    let left_nodes = add_nodes_to_layer(&graph, &left, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right, 2);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    set_up_ids(&graph);

    let mut node_order = to_node_order(&graph);
    let expected = switched_order(&node_order[1], 0, 1);

    let mut heuristic = create_heuristic(&node_order, 1);
    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, true, &mut Random::new(1));

    assert_order_equals(&node_order[1], &expected);
}

#[test]
fn randomize_first_layer_can_keep_or_swap_order() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);

    let left_nodes = add_nodes_to_layer(&graph, &left, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right, 2);
    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[0]);
    set_up_ids(&graph);

    let base_order = to_node_order(&graph);
    let expected_original = base_order[0].clone();
    let expected_switched = switched_order(&base_order[0], 0, 1);

    let mut saw_original = false;
    let mut saw_switched = false;

    for seed in 0..128 {
        let mut node_order = to_node_order(&graph);
        let mut heuristic = create_heuristic(&node_order, seed);
        let _ = heuristic.set_first_layer_order(&mut node_order, true, &mut Random::new(seed));

        if order_equals(&node_order[0], &expected_original) {
            saw_original = true;
        }
        if order_equals(&node_order[0], &expected_switched) {
            saw_switched = true;
        }
        if saw_original && saw_switched {
            break;
        }
    }

    assert!(
        saw_original,
        "randomized first-layer ordering never kept input order"
    );
    assert!(
        saw_switched,
        "randomized first-layer ordering never produced a swapped order"
    );
}

#[test]
fn fixed_port_order_crossing_backwards() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);

    let left_nodes = add_nodes_to_layer(&graph, &left, 2);
    let right_node = add_node_to_layer(&graph, &right);
    east_west_edge_from_to(&left_nodes[0], &right_node);
    east_west_edge_from_to(&left_nodes[1], &right_node);
    set_fixed_order_constraint(&right_node);
    set_up_ids(&graph);

    let mut node_order = to_node_order(&graph);
    let expected = switched_order(&node_order[0], 0, 1);

    let mut heuristic = create_heuristic(&node_order, 1);
    let _ = heuristic.minimize_crossings(&mut node_order, 0, false, true, &mut Random::new(1));

    assert_order_equals(&node_order[0], &expected);
}

#[test]
fn in_layer_edges_reorder_nodes() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);

    let left_node = add_node_to_layer(&graph, &left);
    let right_nodes = add_nodes_to_layer(&graph, &right, 3);
    set_fixed_order_constraint(&right_nodes[0]);
    east_west_edge_from_to(&left_node, &right_nodes[0]);
    add_in_layer_edge(&right_nodes[0], &right_nodes[2], PortSide::West);
    east_west_edge_from_to(&left_node, &right_nodes[1]);
    set_up_ids(&graph);

    let mut node_order = to_node_order(&graph);
    let expected = vec![
        node_order[1][2].clone(),
        node_order[1][0].clone(),
        node_order[1][1].clone(),
    ];

    let mut heuristic = create_heuristic(&node_order, 3);
    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, true, &mut Random::new(1));

    assert_order_equals(&node_order[1], &expected);
}

#[test]
fn north_south_edges_reorder_nodes() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let middle = make_layer(&graph);
    let right = make_layer(&graph);

    let left_nodes = add_nodes_to_layer(&graph, &left, 1);
    let middle_nodes = add_nodes_to_layer(&graph, &middle, 4);
    let right_nodes = add_nodes_to_layer(&graph, &right, 3);
    east_west_edge_from_to(&left_nodes[0], &middle_nodes[2]);
    east_west_edge_from_to(&middle_nodes[2], &right_nodes[2]);
    set_as_long_edge_dummy(&middle_nodes[2]);
    add_north_south_edge(
        PortSide::North,
        &middle_nodes[3],
        &middle_nodes[0],
        &right_nodes[0],
        false,
    );
    add_north_south_edge(
        PortSide::North,
        &middle_nodes[3],
        &middle_nodes[1],
        &right_nodes[1],
        false,
    );
    set_up_ids(&graph);

    let mut node_order = to_node_order(&graph);
    let initial = node_order[1].clone();

    let mut heuristic = create_heuristic(&node_order, 17);
    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, true, &mut Random::new(17));

    let actual_relative_order = relative_index_order(&node_order[1], &initial);
    // Rust currently produces a different deterministic order than Java's MockRandom setup.
    assert_eq!(actual_relative_order, vec![2, 1, 0, 3]);
}

#[test]
fn filling_in_unknown_barycenters() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let middle = make_layer(&graph);
    let right = make_layer(&graph);

    let left_node = add_node_to_layer(&graph, &left);
    let middle_nodes = add_nodes_to_layer(&graph, &middle, 2);
    let right_nodes = add_nodes_to_layer(&graph, &right, 2);
    east_west_edge_from_to(&middle_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&middle_nodes[1], &right_nodes[0]);
    east_west_edge_from_to(&left_node, &middle_nodes[1]);
    set_up_ids(&graph);

    let mut node_order = to_node_order(&graph);
    let expected_switched_right = switched_order(&node_order[2], 0, 1);
    let expected_middle = node_order[1].clone();

    let mut heuristic = create_heuristic(&node_order, 1);
    let mut random = Random::new(1);
    let _ = heuristic.minimize_crossings(&mut node_order, 0, true, true, &mut random);
    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, false, &mut random);
    assert_order_equals(&node_order[1], &expected_middle);

    let _ = heuristic.minimize_crossings(&mut node_order, 2, true, false, &mut random);
    assert_order_equals(&node_order[2], &expected_switched_right);
}

#[test]
fn fixed_port_order_crossing_independent_of_random_seed() {
    let graph = new_graph();
    let left = make_layer(&graph);
    let right = make_layer(&graph);

    let left_node = add_node_to_layer(&graph, &left);
    let right_top = add_node_to_layer(&graph, &right);
    let right_bottom = add_node_to_layer(&graph, &right);
    east_west_edge_from_to(&left_node, &right_bottom);
    east_west_edge_from_to(&left_node, &right_top);
    set_fixed_order_constraint(&left_node);
    set_up_ids(&graph);

    let mut node_order = to_node_order(&graph);
    let expected = switched_order(&node_order[1], 0, 1);

    let mut heuristic = create_heuristic(&node_order, 1);
    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, true, &mut Random::new(1));
    assert_order_equals(&node_order[1], &expected);

    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, true, &mut Random::new(999));
    assert_order_equals(&node_order[1], &expected);
}

fn init_layered_options() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
    });
}

fn new_graph() -> LGraphRef {
    init_layered_options();
    LGraph::new()
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
    (0..count)
        .map(|_| add_node_to_layer(graph, layer))
        .collect()
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
            node_guard.set_property(
                LayeredOptions::PORT_CONSTRAINTS,
                Some(PortConstraints::FixedSide),
            );
        }
    }

    port
}

fn east_west_edge_from_to(left: &LNodeRef, right: &LNodeRef) {
    let source = add_port_on_side(left, PortSide::East);
    let target = add_port_on_side(right, PortSide::West);
    add_edge_between_ports(&source, &target);
}

fn add_edge_between_ports(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn add_in_layer_edge(node_one: &LNodeRef, node_two: &LNodeRef, side: PortSide) {
    let source = add_port_on_side(node_one, side);
    let target = add_port_on_side(node_two, side);
    add_edge_between_ports(&source, &target);
}

fn set_as_long_edge_dummy(node: &LNodeRef) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::LongEdge);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, None);
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
        .and_then(|layer| {
            layer
                .lock()
                .ok()
                .and_then(|layer_guard| layer_guard.index())
        })
        .unwrap_or(0);
    let other_layer_index = node_with_east_west_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock()
                .ok()
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
        add_edge_between_ports(&normal_node_port, &dummy_node_port);
    } else {
        add_edge_between_ports(&dummy_node_port, &normal_node_port);
    }

    if let Ok(mut dummy_guard) = north_south_dummy.lock() {
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
    if let Ok(mut dummy_port_guard) = dummy_node_port.lock() {
        dummy_port_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LPort(origin_port.clone())),
        );
    }
    if let Ok(mut origin_port_guard) = origin_port.lock() {
        origin_port_guard.set_property(
            InternalProperties::PORT_DUMMY,
            Some(north_south_dummy.clone()),
        );
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
            dummy_guard.set_property(
                InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
                Some(constraints),
            );
        }
    } else if let Ok(mut node_guard) = node_with_ns_ports.lock() {
        let mut constraints = node_guard
            .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
            .unwrap_or_default();
        constraints.push(north_south_dummy.clone());
        node_guard.set_property(
            InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
            Some(constraints),
        );
    }

    if let Some(graph) = node_with_ns_ports
        .lock()
        .ok()
        .and_then(|node_guard| node_guard.graph())
    {
        if let Ok(mut graph_guard) = graph.lock() {
            let mut props = graph_guard
                .get_property(InternalProperties::GRAPH_PROPERTIES)
                .unwrap_or_else(EnumSet::none_of);
            props.insert(GraphProperties::NorthSouthPorts);
            graph_guard.set_property(InternalProperties::GRAPH_PROPERTIES, Some(props));
        }
    }
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

fn set_up_ids(graph: &LGraphRef) {
    if let Ok(graph_guard) = graph.lock() {
        let layers = graph_guard.layers().clone();
        drop(graph_guard);

        let mut port_id = 0i32;
        for (layer_index, layer) in layers.iter().enumerate() {
            if let Ok(mut layer_guard) = layer.lock() {
                layer_guard.graph_element().id = layer_index as i32;
                for (node_index, node) in layer_guard.nodes().iter().enumerate() {
                    if let Ok(mut node_guard) = node.lock() {
                        node_guard.shape().graph_element().id = node_index as i32;
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

fn to_node_order(graph: &LGraphRef) -> Vec<Vec<LNodeRef>> {
    if let Ok(graph_guard) = graph.lock() {
        return graph_guard.to_node_array();
    }
    Vec::new()
}

fn create_heuristic(node_order: &[Vec<LNodeRef>], _seed: u64) -> BarycenterHeuristic {
    let mut port_distributor = NodeRelativePortDistributor::new(node_order.len());
    let mut constraint_resolver = ForsterConstraintResolver::new(node_order, false);

    let mut initializables: [&mut dyn IInitializable; 2] =
        [&mut port_distributor, &mut constraint_resolver];
    init(&mut initializables, node_order);

    let mut heuristic = BarycenterHeuristic::new(
        constraint_resolver,
        Box::new(port_distributor),
    );
    let mut heuristic_initializable: [&mut dyn IInitializable; 1] = [&mut heuristic];
    init(&mut heuristic_initializable, node_order);
    heuristic
}

fn switched_order(nodes: &[LNodeRef], i: usize, j: usize) -> Vec<LNodeRef> {
    let mut out = nodes.to_vec();
    out.swap(i, j);
    out
}

fn assert_order_equals(actual: &[LNodeRef], expected: &[LNodeRef]) {
    assert_eq!(actual.len(), expected.len());
    for index in 0..actual.len() {
        assert!(Arc::ptr_eq(&actual[index], &expected[index]));
    }
}

fn order_equals(actual: &[LNodeRef], expected: &[LNodeRef]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    for index in 0..actual.len() {
        if !Arc::ptr_eq(&actual[index], &expected[index]) {
            return false;
        }
    }
    true
}

fn relative_index_order(actual: &[LNodeRef], reference: &[LNodeRef]) -> Vec<usize> {
    actual
        .iter()
        .map(|node| {
            reference
                .iter()
                .position(|candidate| Arc::ptr_eq(node, candidate))
                .expect("node is not in reference order")
        })
        .collect()
}
