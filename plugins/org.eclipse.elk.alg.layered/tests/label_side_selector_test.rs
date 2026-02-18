use std::sync::{Arc, Mutex};

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LLabel, LNode, LNodeRef, LPort, LPortRef, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LabelSideSelector;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    EdgeLabelSideSelection, InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_single_layer() -> (LGraphRef, Arc<Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef) -> LPortRef {
    let port = LPort::new();
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(
    source: &LPortRef,
    target: &LPortRef,
) -> org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

#[test]
fn test_removed_nodes() {
    let (graph, layer) = graph_with_single_layer();
    graph.lock().expect("graph lock").set_property(
        LayeredOptions::EDGE_LABELS_SIDE_SELECTION,
        Some(EdgeLabelSideSelection::AlwaysUp),
    );

    let source = add_node(&graph, &layer);
    let target = add_node(&graph, &layer);
    let source_port = add_port(&source);
    let target_port = add_port(&target);
    let edge = connect(&source_port, &target_port);

    let label = Arc::new(Mutex::new(LLabel::with_text("edge-label")));
    edge.lock()
        .expect("edge lock")
        .labels_mut()
        .push(label.clone());

    let mut selector = LabelSideSelector;
    let mut monitor = NullElkProgressMonitor;
    selector.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let side = label
        .lock()
        .expect("label lock")
        .get_property(InternalProperties::LABEL_SIDE)
        .unwrap_or(LabelSide::Unknown);
    assert_ne!(side, LabelSide::Unknown);
}
