use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::CommentNodeMarginCalculator;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

const TOLERANCE: f64 = 1e-6;

fn new_graph_with_layer() -> (LGraphRef, LayerRef) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    {
        let mut graph_guard = graph.lock();        graph_guard.layers_mut().push(layer.clone());
    }
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &LayerRef, width: f64, height: f64) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().size().x = width;
        node_guard.shape().size().y = height;
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn run_processor(graph: &LGraphRef) {
    let mut processor = CommentNodeMarginCalculator;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn top_and_bottom_comment_margins_are_applied() {
    let (graph, layer) = new_graph_with_layer();
    {
        let mut graph_guard = graph.lock();        graph_guard.set_property(LayeredOptions::SPACING_COMMENT_COMMENT, Some(2.0));
        graph_guard.set_property(LayeredOptions::SPACING_COMMENT_NODE, Some(3.0));
    }

    let node = add_node(&graph, &layer, 50.0, 20.0);
    let top_a = add_node(&graph, &layer, 30.0, 8.0);
    let top_b = add_node(&graph, &layer, 20.0, 6.0);
    let bottom = add_node(&graph, &layer, 10.0, 5.0);

    {
        let mut node_guard = node.lock();        node_guard.set_property(InternalProperties::TOP_COMMENTS, Some(vec![top_a, top_b]));
        node_guard.set_property(InternalProperties::BOTTOM_COMMENTS, Some(vec![bottom]));
    }

    run_processor(&graph);

    let mut node_guard = node.lock();    assert!((node_guard.margin().top - 11.0).abs() < TOLERANCE);
    assert!((node_guard.margin().bottom - 8.0).abs() < TOLERANCE);
}

#[test]
fn node_without_comments_keeps_existing_margins() {
    let (graph, layer) = new_graph_with_layer();
    let node = add_node(&graph, &layer, 40.0, 10.0);

    {
        let mut node_guard = node.lock();        node_guard.margin().top = 1.0;
        node_guard.margin().right = 2.0;
        node_guard.margin().bottom = 3.0;
        node_guard.margin().left = 4.0;
    }

    run_processor(&graph);

    let mut node_guard = node.lock();    assert!((node_guard.margin().top - 1.0).abs() < TOLERANCE);
    assert!((node_guard.margin().right - 2.0).abs() < TOLERANCE);
    assert!((node_guard.margin().bottom - 3.0).abs() < TOLERANCE);
    assert!((node_guard.margin().left - 4.0).abs() < TOLERANCE);
}
