use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LayerConstraintPostprocessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayerConstraint, LayeredMetaDataProvider, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::data::LayoutMetaDataService;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
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

fn add_node(graph: &LGraphRef, layer: &LayerRef, constraint: LayerConstraint) -> LNodeRef {
    let node = LNode::new(graph);
    if constraint != LayerConstraint::None {
        node.lock()
            
            .set_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT, Some(constraint));
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef) -> LPortRef {
    let port = LPort::new();
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn run_processor(graph: &LGraphRef) {
    LayoutMetaDataService::get_instance()
        .register_layout_meta_data_provider(&LayeredMetaDataProvider);
    let mut processor = LayerConstraintPostprocessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn layer_constraint_postprocessor_moves_first_and_last_nodes_to_outer_layers() {
    let (graph, layers) = new_graph_with_layers(2);
    let first_layer = layers[0].clone();
    let last_layer = layers[1].clone();

    let _anchor_first = add_node(&graph, &first_layer, LayerConstraint::None);
    let _anchor_last = add_node(&graph, &last_layer, LayerConstraint::None);
    let first_node = add_node(&graph, &last_layer, LayerConstraint::First);
    let last_node = add_node(&graph, &first_layer, LayerConstraint::Last);

    run_processor(&graph);

    let first_node_layer = first_node
        .lock()
        
        .layer()
        .expect("layer");
    let last_node_layer = last_node
        .lock()
        
        .layer()
        .expect("layer");
    assert!(Arc::ptr_eq(&first_node_layer, &first_layer));
    assert!(Arc::ptr_eq(&last_node_layer, &last_layer));
}

#[test]
fn layer_constraint_postprocessor_restores_hidden_nodes_and_detached_edges() {
    let (graph, layers) = new_graph_with_layers(1);
    let main_layer = layers[0].clone();
    let opposite = add_node(&graph, &main_layer, LayerConstraint::None);

    let hidden = LNode::new(&graph);
    hidden.lock().set_property(
        LayeredOptions::LAYERING_LAYER_CONSTRAINT,
        Some(LayerConstraint::FirstSeparate),
    );

    let hidden_port = add_port(&hidden);
    let opposite_port = add_port(&opposite);
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(hidden_port));
    LEdge::set_target(&edge, Some(opposite_port.clone()));
    LEdge::set_target(&edge, None);
    edge.lock().set_property(
        InternalProperties::ORIGINAL_OPPOSITE_PORT,
        Some(opposite_port.clone()),
    );

    graph
        .lock()
        
        .set_property(InternalProperties::HIDDEN_NODES, Some(vec![hidden.clone()]));

    run_processor(&graph);

    let layers_after = graph.lock().layers().clone();
    let first_layer = layers_after.first().cloned().expect("first layer");
    let nodes_in_first = first_layer
        .lock()
        
        .nodes()
        .clone();
    assert!(nodes_in_first.iter().any(|node| Arc::ptr_eq(node, &hidden)));

    let target = edge.lock().target().expect("target");
    assert!(Arc::ptr_eq(&target, &opposite_port));
}

fn add_hidden_node(
    graph: &LGraphRef,
    constraint: LayerConstraint,
    node_type: NodeType,
) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut ng = node.lock().expect("node lock");
        ng.set_property(LayeredOptions::LAYERING_LAYER_CONSTRAINT, Some(constraint));
        ng.set_node_type(node_type);
    }
    node
}

#[test]
fn postprocessor_separates_external_port_from_separate_layer() {
    let (graph, layers) = new_graph_with_layers(1);
    let main_layer = layers[0].clone();
    let _anchor = add_node(&graph, &main_layer, LayerConstraint::None);

    // Create a FIRST_SEPARATE normal node and a FIRST_SEPARATE external port node
    let first_sep_normal = add_hidden_node(&graph, LayerConstraint::FirstSeparate, NodeType::Normal);
    let first_sep_ext_port =
        add_hidden_node(&graph, LayerConstraint::FirstSeparate, NodeType::ExternalPort);

    // Create a LAST_SEPARATE normal node and a LAST_SEPARATE external port node
    let last_sep_normal = add_hidden_node(&graph, LayerConstraint::LastSeparate, NodeType::Normal);
    let last_sep_ext_port =
        add_hidden_node(&graph, LayerConstraint::LastSeparate, NodeType::ExternalPort);

    graph.lock().expect("graph lock").set_property(
        InternalProperties::HIDDEN_NODES,
        Some(vec![
            first_sep_normal.clone(),
            first_sep_ext_port.clone(),
            last_sep_normal.clone(),
            last_sep_ext_port.clone(),
        ]),
    );

    run_processor(&graph);

    let layers_after = graph.lock().expect("graph lock").layers().clone();

    // Should have at least 5 layers:
    // [first_ext_port_layer] [first_separate_layer] [main] [last_separate_layer] [last_ext_port_layer]
    assert!(
        layers_after.len() >= 5,
        "expected at least 5 layers, got {}",
        layers_after.len()
    );

    // First layer should contain the external port node
    let first_layer_nodes = layers_after[0]
        .lock()
        .expect("first layer lock")
        .nodes()
        .clone();
    assert!(
        first_layer_nodes
            .iter()
            .any(|n| Arc::ptr_eq(n, &first_sep_ext_port)),
        "first external port should be in outermost first layer"
    );

    // Second layer should contain the normal FIRST_SEPARATE node
    let second_layer_nodes = layers_after[1]
        .lock()
        .expect("second layer lock")
        .nodes()
        .clone();
    assert!(
        second_layer_nodes
            .iter()
            .any(|n| Arc::ptr_eq(n, &first_sep_normal)),
        "first separate normal node should be in second layer"
    );

    // Second-to-last layer should contain the normal LAST_SEPARATE node
    let second_to_last_nodes = layers_after[layers_after.len() - 2]
        .lock()
        .expect("second to last layer lock")
        .nodes()
        .clone();
    assert!(
        second_to_last_nodes
            .iter()
            .any(|n| Arc::ptr_eq(n, &last_sep_normal)),
        "last separate normal node should be in second-to-last layer"
    );

    // Last layer should contain the external port node
    let last_layer_nodes = layers_after[layers_after.len() - 1]
        .lock()
        .expect("last layer lock")
        .nodes()
        .clone();
    assert!(
        last_layer_nodes
            .iter()
            .any(|n| Arc::ptr_eq(n, &last_sep_ext_port)),
        "last external port should be in outermost last layer"
    );
}
