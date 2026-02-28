use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::InLayerConstraintProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InLayerConstraint, InternalProperties,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_layer() -> (LGraphRef, Arc<Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node_with_constraint(
    graph: &LGraphRef,
    layer: &Arc<Mutex<Layer>>,
    constraint: InLayerConstraint,
) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(InternalProperties::IN_LAYER_CONSTRAINT, Some(constraint));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn run_processor(graph: &LGraphRef) {
    let mut processor = InLayerConstraintProcessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock().expect("graph lock");
    processor.process(&mut graph_guard, &mut monitor);
}

fn layer_nodes(layer: &Arc<Mutex<Layer>>) -> Vec<LNodeRef> {
    layer.lock().expect("layer lock").nodes().clone()
}

#[test]
fn in_layer_constraint_processor_moves_top_to_front_and_bottom_to_back() {
    let (graph, layer) = graph_with_layer();
    let none1 = add_node_with_constraint(&graph, &layer, InLayerConstraint::None);
    let top1 = add_node_with_constraint(&graph, &layer, InLayerConstraint::Top);
    let none2 = add_node_with_constraint(&graph, &layer, InLayerConstraint::None);
    let top2 = add_node_with_constraint(&graph, &layer, InLayerConstraint::Top);
    let bottom1 = add_node_with_constraint(&graph, &layer, InLayerConstraint::Bottom);
    let none3 = add_node_with_constraint(&graph, &layer, InLayerConstraint::None);
    let bottom2 = add_node_with_constraint(&graph, &layer, InLayerConstraint::Bottom);

    run_processor(&graph);

    let nodes = layer_nodes(&layer);
    assert!(Arc::ptr_eq(&nodes[0], &top1));
    assert!(Arc::ptr_eq(&nodes[1], &top2));
    assert!(Arc::ptr_eq(&nodes[2], &none1));
    assert!(Arc::ptr_eq(&nodes[3], &none2));
    assert!(Arc::ptr_eq(&nodes[4], &none3));
    assert!(Arc::ptr_eq(&nodes[5], &bottom1));
    assert!(Arc::ptr_eq(&nodes[6], &bottom2));
}

#[test]
fn in_layer_constraint_processor_keeps_order_when_constraints_are_already_grouped() {
    let (graph, layer) = graph_with_layer();
    let top = add_node_with_constraint(&graph, &layer, InLayerConstraint::Top);
    let none = add_node_with_constraint(&graph, &layer, InLayerConstraint::None);
    let bottom = add_node_with_constraint(&graph, &layer, InLayerConstraint::Bottom);

    run_processor(&graph);

    let nodes = layer_nodes(&layer);
    assert!(Arc::ptr_eq(&nodes[0], &top));
    assert!(Arc::ptr_eq(&nodes[1], &none));
    assert!(Arc::ptr_eq(&nodes[2], &bottom));
}
