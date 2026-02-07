use std::sync::OnceLock;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredMetaDataProvider, LayeredOptions, OrderingStrategy,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::counting::i_initializable::{
    init, IInitializable,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::model_order_barycenter_heuristic::ModelOrderBarycenterHeuristic;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::p3order::node_relative_port_distributor::NodeRelativePortDistributor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::ElkReflect;

#[test]
fn test_model_order_respected() {
    let graph = create_graph();
    let mut node_order = to_node_order(&graph);
    let mut heuristic = create_heuristic(&node_order, 7);

    let _ = heuristic.minimize_crossings(&mut node_order, 1, true, true);

    let mut model_order = -1;
    for node in &node_order[1] {
        if let Ok(mut node_guard) = node.lock() {
            if let Some(new_model_order) = node_guard.get_property(InternalProperties::MODEL_ORDER) {
                assert!(
                    new_model_order > model_order,
                    "model order regression: node has {}, previous was {}",
                    new_model_order,
                    model_order
                );
                model_order = new_model_order;
            }
        }
    }
}

fn init_layered_options() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let service = LayoutMetaDataService::get_instance();
        service.register_layout_meta_data_provider(&LayeredMetaDataProvider);
        ElkReflect::register(Some(|| LayerConstraint::None), Some(|value: &LayerConstraint| *value));
    });
}

fn create_graph() -> LGraphRef {
    init_layered_options();
    let graph = LGraph::new();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::CONSIDER_MODEL_ORDER_STRATEGY,
            Some(OrderingStrategy::NodesAndEdges),
        );
        graph_guard.set_property(
            LayeredOptions::CROSSING_MINIMIZATION_FORCE_NODE_MODEL_ORDER,
            Some(true),
        );
    }

    let left = make_layer(&graph);
    let right = make_layer(&graph);

    let left_nodes = [
        add_node_to_layer(&graph, &left, None),
        add_node_to_layer(&graph, &left, None),
        add_node_to_layer(&graph, &left, None),
    ];
    let right_nodes = [
        add_node_to_layer(&graph, &right, Some(2)),
        add_node_to_layer(&graph, &right, Some(0)),
        add_node_to_layer(&graph, &right, Some(1)),
    ];

    east_west_edge_from_to(&left_nodes[0], &right_nodes[1]);
    east_west_edge_from_to(&left_nodes[1], &right_nodes[2]);
    east_west_edge_from_to(&left_nodes[2], &right_nodes[0]);

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

fn add_node_to_layer(graph: &LGraphRef, layer: &LayerRef, model_order: Option<i32>) -> LNodeRef {
    let node = LNode::new(graph);
    if let Ok(mut node_guard) = node.lock() {
        node_guard.set_node_type(NodeType::Normal);
        node_guard.set_property(InternalProperties::IN_LAYER_LAYOUT_UNIT, Some(node.clone()));
        node_guard.set_property(InternalProperties::MODEL_ORDER, model_order);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port_on_side(node: &LNodeRef, side: PortSide) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LPortRef {
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
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source));
    LEdge::set_target(&edge, Some(target));
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

fn create_heuristic(node_order: &[Vec<LNodeRef>], seed: u64) -> ModelOrderBarycenterHeuristic {
    let mut port_distributor = NodeRelativePortDistributor::new(node_order.len());
    let mut constraint_resolver = ForsterConstraintResolver::new(node_order, false);

    let mut initializables: [&mut dyn IInitializable; 2] =
        [&mut port_distributor, &mut constraint_resolver];
    init(&mut initializables, node_order);

    let mut heuristic = ModelOrderBarycenterHeuristic::new(
        constraint_resolver,
        Random::new(seed),
        Box::new(port_distributor),
    );
    let mut heuristic_initializable: [&mut dyn IInitializable; 1] = [&mut heuristic];
    init(&mut heuristic_initializable, node_order);
    heuristic
}
