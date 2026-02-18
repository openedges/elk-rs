use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CrossingMinimizationStrategy, InternalProperties,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase_factory::ILayoutPhaseFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_single_layer_graph() -> (LGraphRef, LayerRef) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node_to_layer(
    graph: &LGraphRef,
    layer: &LayerRef,
    node_type: NodeType,
    y: f64,
    height: f64,
) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_node_type(node_type);
        node_guard.shape().position().y = y;
        node_guard.shape().size().y = height;
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn run_interactive_crossing_minimization(graph: &LGraphRef) {
    LayoutMetaDataService::get_instance();
    let mut phase = CrossingMinimizationStrategy::Interactive.create_phase();
    let mut monitor = NullElkProgressMonitor;
    phase.process(&mut graph.lock().expect("graph lock"), &mut monitor);
}

#[test]
fn interactive_strategy_creates_interactive_crossing_minimizer_phase() {
    let phase = CrossingMinimizationStrategy::Interactive.create_phase();
    assert!(
        phase
            .type_name()
            .contains("interactive_crossing_minimizer::InteractiveCrossingMinimizer"),
        "unexpected phase type: {}",
        phase.type_name()
    );
}

#[test]
fn interactive_crossing_minimizer_sorts_layer_and_sets_dummy_position() {
    let (graph, layer) = new_single_layer_graph();
    let normal = add_node_to_layer(&graph, &layer, NodeType::Normal, 100.0, 20.0);
    let long_edge_dummy = add_node_to_layer(&graph, &layer, NodeType::LongEdge, 20.0, 10.0);

    // Start in reverse order so the phase must actively sort by previous y positions.
    {
        let mut layer_guard = layer.lock().expect("layer lock");
        layer_guard.nodes_mut().clear();
        layer_guard.nodes_mut().push(normal.clone());
        layer_guard.nodes_mut().push(long_edge_dummy.clone());
    }

    run_interactive_crossing_minimization(&graph);

    let ordered_nodes = layer.lock().expect("layer lock").nodes().clone();
    assert!(Arc::ptr_eq(&ordered_nodes[0], &long_edge_dummy));
    assert!(Arc::ptr_eq(&ordered_nodes[1], &normal));

    let original_dummy_pos = long_edge_dummy
        .lock()
        .expect("dummy node lock")
        .get_property(InternalProperties::ORIGINAL_DUMMY_NODE_POSITION)
        .expect("missing original dummy position");
    assert!(
        (original_dummy_pos - 25.0).abs() < 1e-6,
        "expected dummy y center 25.0 but got {original_dummy_pos}"
    );
}

#[test]
fn interactive_crossing_minimizer_respects_in_layer_successor_constraints_on_ties() {
    let (graph, layer) = new_single_layer_graph();
    let node_a = add_node_to_layer(&graph, &layer, NodeType::Normal, 0.0, 0.0);
    let node_b = add_node_to_layer(&graph, &layer, NodeType::Normal, 0.0, 0.0);

    {
        node_a.lock().expect("node_a lock").set_property(
            InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS,
            Some(vec![node_b.clone()]),
        );
        let mut layer_guard = layer.lock().expect("layer lock");
        layer_guard.nodes_mut().clear();
        layer_guard.nodes_mut().push(node_b.clone());
        layer_guard.nodes_mut().push(node_a.clone());
    }

    run_interactive_crossing_minimization(&graph);

    let ordered_nodes = layer.lock().expect("layer lock").nodes().clone();
    assert!(Arc::ptr_eq(&ordered_nodes[0], &node_a));
    assert!(Arc::ptr_eq(&ordered_nodes[1], &node_b));
}
