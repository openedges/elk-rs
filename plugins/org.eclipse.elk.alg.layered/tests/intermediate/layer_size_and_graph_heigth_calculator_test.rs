use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LayerSizeAndGraphHeightCalculator;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

const TOLERANCE: f64 = 1e-6;

fn graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    {
        let mut graph_guard = graph.lock();        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_node(
    graph: &LGraphRef,
    layer: &LayerRef,
    y: f64,
    w: f64,
    h: f64,
    margin_top: f64,
    margin_bottom: f64,
) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.shape().position().y = y;
        node_guard.shape().size().x = w;
        node_guard.shape().size().y = h;
        node_guard.margin().top = margin_top;
        node_guard.margin().bottom = margin_bottom;
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

#[test]
fn layer_size_and_graph_heigth_calculator_bounds_all_nodes() {
    let (graph, layers) = graph_with_layers(2);
    let first = layers[0].clone();
    let second = layers[1].clone();

    add_node(&graph, &first, -5.0, 20.0, 12.0, 1.0, 2.0);
    add_node(&graph, &second, 30.0, 25.0, 10.0, 0.5, 2.0);

    let mut processor = LayerSizeAndGraphHeightCalculator;
    let mut monitor = NullElkProgressMonitor;
    processor.process(&mut graph.lock(), &mut monitor);

    let graph_guard = graph.lock();    let top = -graph_guard.offset_ref().y;
    let bottom = graph_guard.size_ref().y - graph_guard.offset_ref().y;
    let layers = graph_guard.layers().clone();
    drop(graph_guard);

    for layer in layers {
        let nodes = layer.lock().nodes().clone();
        for node in nodes {
            let mut node_guard = node.lock();            let node_top = node_guard.shape().position_ref().y - node_guard.margin().top;
            let node_bottom = node_guard.shape().position_ref().y
                + node_guard.shape().size_ref().y
                + node_guard.margin().bottom;
            assert!(node_top > top || (node_top - top).abs() < TOLERANCE);
            assert!(node_bottom < bottom || (node_bottom - bottom).abs() < TOLERANCE);
        }
    }
}
