use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::ReversedEdgeRestorer;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_layer() -> (LGraphRef, Arc<std::sync::Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &Arc<std::sync::Mutex<Layer>>) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef) -> LPortRef {
    let port = LPort::new();
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef, reversed: bool) -> LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge.lock()
        .expect("edge lock")
        .set_property(InternalProperties::REVERSED, Some(reversed));
    edge
}

fn run_direct(graph: &LGraphRef) {
    let mut processor = ReversedEdgeRestorer;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock().expect("graph lock");
    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn reversed_edge_restorer_reverses_marked_edge_direction() {
    let (graph, layer) = graph_with_layer();
    let source_node = add_node(&graph, &layer);
    let target_node = add_node(&graph, &layer);
    let source_port = add_port(&source_node);
    let target_port = add_port(&target_node);
    let edge = connect(&source_port, &target_port, true);

    run_direct(&graph);

    let source = edge.lock().expect("edge lock").source().expect("source");
    let target = edge.lock().expect("edge lock").target().expect("target");
    let reversed = edge
        .lock()
        .expect("edge lock")
        .get_property(InternalProperties::REVERSED)
        .unwrap_or(true);
    assert!(Arc::ptr_eq(&source, &target_port));
    assert!(Arc::ptr_eq(&target, &source_port));
    assert!(!reversed);
}
