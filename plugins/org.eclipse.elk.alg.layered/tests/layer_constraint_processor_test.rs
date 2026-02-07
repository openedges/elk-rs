use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    LayerConstraintPostprocessor, LayerConstraintPreprocessor,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn init_layered_metadata() {
    LayoutMetaDataService::get_instance().register_layout_meta_data_provider(&LayeredMetaDataProvider);
}

fn new_graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    {
        let mut graph_guard = graph.lock().expect("graph lock");
        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_node(graph: &LGraphRef, layer: Option<&LayerRef>, constraint: LayerConstraint) -> LNodeRef {
    let node = LNode::new(graph);
    if constraint != LayerConstraint::None {
        node.lock()
            .expect("node lock")
            .set_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT, Some(constraint));
    }
    if let Some(layer) = layer {
        LNode::set_layer(&node, Some(layer.clone()));
    } else {
        graph
            .lock()
            .expect("graph lock")
            .layerless_nodes_mut()
            .push(node.clone());
    }
    node
}

#[test]
fn layer_constraint_processor_hides_first_separate_nodes() {
    init_layered_metadata();
    let (graph, layers) = new_graph_with_layers(1);
    let _anchor = add_node(&graph, Some(&layers[0]), LayerConstraint::None);
    let hidden = add_node(&graph, None, LayerConstraint::FirstSeparate);

    let mut pre = LayerConstraintPreprocessor;
    let mut monitor = NullElkProgressMonitor;
    pre.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let hidden_nodes = graph
        .lock()
        .expect("graph lock")
        .get_property(InternalProperties::HIDDEN_NODES)
        .unwrap_or_default();
    assert!(
        hidden_nodes
            .iter()
            .any(|node| Arc::ptr_eq(node, &hidden)),
        "first-separate node must be moved to hidden node list"
    );
}

#[test]
fn layer_constraint_processor_moves_first_and_last_after_postprocessing() {
    init_layered_metadata();
    let (graph, layers) = new_graph_with_layers(2);
    let first_layer = layers[0].clone();
    let last_layer = layers[1].clone();

    let _anchor_first = add_node(&graph, Some(&first_layer), LayerConstraint::None);
    let _anchor_last = add_node(&graph, Some(&last_layer), LayerConstraint::None);
    let first_node = add_node(&graph, Some(&last_layer), LayerConstraint::First);
    let last_node = add_node(&graph, Some(&first_layer), LayerConstraint::Last);

    let mut post = LayerConstraintPostprocessor;
    let mut monitor = NullElkProgressMonitor;
    post.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let first_node_layer = first_node
        .lock()
        .expect("first node lock")
        .layer()
        .expect("layer");
    let last_node_layer = last_node
        .lock()
        .expect("last node lock")
        .layer()
        .expect("layer");
    assert!(Arc::ptr_eq(&first_node_layer, &first_layer));
    assert!(Arc::ptr_eq(&last_node_layer, &last_layer));
}
