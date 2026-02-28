use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::layer_sweep_crossing_minimizer::{
    CrossMinType, LayerSweepCrossingMinimizer,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::hierarchy_handling::HierarchyHandling;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{BasicProgressMonitor, Random};

#[test]
fn ensure_layer_assignment_unchanged() {
    ensure_layer_assignment_unchanged_for(CrossMinType::Barycenter);
}

#[test]
fn ensure_layer_assignment_unchanged_greedy_switch() {
    ensure_layer_assignment_unchanged_for(CrossMinType::TwoSidedGreedySwitch);
}

fn ensure_layer_assignment_unchanged_for(cross_min_type: CrossMinType) {
    let graph = create_test_graph();
    let before_assignment = record_layer_assignment(&graph);

    let mut minimizer = LayerSweepCrossingMinimizer::new(cross_min_type);
    let mut monitor = BasicProgressMonitor::new();
    if let Ok(mut graph_guard) = graph.lock() {
        minimizer.process(&mut graph_guard, &mut monitor);
    }

    verify_layer_assignment(&graph, &before_assignment);
}

fn init_layered_options() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
    });
}

fn create_test_graph() -> LGraphRef {
    init_layered_options();

    let graph = LGraph::new();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(
            LayeredOptions::HIERARCHY_HANDLING,
            Some(HierarchyHandling::IncludeChildren),
        );
        graph_guard.set_property(InternalProperties::RANDOM, Some(Random::new(42)));
    }

    let left = make_layer(&graph);
    let middle = make_layer(&graph);
    let right = make_layer(&graph);

    let n1 = add_node_to_layer(&graph, &left);
    let n2 = add_node_to_layer(&graph, &left);
    let n3 = add_node_to_layer(&graph, &middle);
    let n4 = add_node_to_layer(&graph, &middle);
    let n5 = add_node_to_layer(&graph, &middle);
    let n6 = add_node_to_layer(&graph, &right);
    let n7 = add_node_to_layer(&graph, &right);

    east_west_edge_from_to(&n1, &n4);
    east_west_edge_from_to(&n2, &n3);
    east_west_edge_from_to(&n1, &n5);
    east_west_edge_from_to(&n3, &n7);
    east_west_edge_from_to(&n4, &n6);
    east_west_edge_from_to(&n5, &n7);

    set_up_ids(&graph);
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
    let left_port = add_port_on_side(left, PortSide::East);
    let right_port = add_port_on_side(right, PortSide::West);
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(left_port));
    LEdge::set_target(&edge, Some(right_port));
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

fn record_layer_assignment(graph: &LGraphRef) -> HashMap<usize, HashSet<usize>> {
    let mut assignment = HashMap::new();
    if let Ok(graph_guard) = graph.lock() {
        for layer in graph_guard.layers() {
            let layer_key = Arc::as_ptr(layer) as usize;
            let nodes = layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().clone())
                .unwrap_or_default();
            let node_keys = nodes
                .iter()
                .map(|node| Arc::as_ptr(node) as usize)
                .collect();
            assignment.insert(layer_key, node_keys);
        }
    }
    assignment
}

fn verify_layer_assignment(graph: &LGraphRef, expected: &HashMap<usize, HashSet<usize>>) {
    if let Ok(graph_guard) = graph.lock() {
        for layer in graph_guard.layers() {
            let layer_key = Arc::as_ptr(layer) as usize;
            let expected_nodes = expected.get(&layer_key).expect("missing expected layer");
            let actual_nodes: HashSet<usize> = layer
                .lock()
                .ok()
                .map(|layer_guard| {
                    layer_guard
                        .nodes()
                        .iter()
                        .map(|node| Arc::as_ptr(node) as usize)
                        .collect()
                })
                .unwrap_or_default();
            assert_eq!(actual_nodes, *expected_nodes);
        }
    }
}
