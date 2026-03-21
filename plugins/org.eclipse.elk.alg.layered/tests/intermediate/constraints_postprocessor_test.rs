
use crate::common::issue_support::init_layered_options;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::ConstraintsPostprocessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);

    if let Some(mut graph_guard) = graph.lock_ok() {
        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }

    (graph, layers)
}

fn add_node_to_layer(graph: &LGraphRef, layer: &LayerRef, node_type: NodeType) -> LNodeRef {
    let node = LNode::new(graph);
    if let Some(mut node_guard) = node.lock_ok() {
        node_guard.set_node_type(node_type);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn node_ids(node: &LNodeRef) -> (i32, i32) {
    let Some(mut node_guard) = node.lock_ok() else {
        return (-1, -1);
    };

    let layer_id = node_guard
        .get_property(LayeredOptions::LAYERING_LAYER_ID)
        .unwrap_or(-1);
    let position_id = node_guard
        .get_property(LayeredOptions::CROSSING_MINIMIZATION_POSITION_ID)
        .unwrap_or(-1);
    (layer_id, position_id)
}

#[test]
fn assigns_layer_and_position_ids_only_to_normal_nodes() {
    init_layered_options();
    let (graph, layers) = new_graph_with_layers(3);

    let label_l0 = add_node_to_layer(&graph, &layers[0], NodeType::Label);
    let normal_l0_a = add_node_to_layer(&graph, &layers[0], NodeType::Normal);
    let normal_l0_b = add_node_to_layer(&graph, &layers[0], NodeType::Normal);
    let label_l1 = add_node_to_layer(&graph, &layers[1], NodeType::Label);
    let normal_l2 = add_node_to_layer(&graph, &layers[2], NodeType::Normal);

    let mut processor = ConstraintsPostprocessor;
    let mut monitor = NullElkProgressMonitor;
    if let Some(mut graph_guard) = graph.lock_ok() {
        processor.process(&mut graph_guard, &mut monitor);
    }

    assert_eq!(node_ids(&label_l0), (-1, -1));
    assert_eq!(node_ids(&normal_l0_a), (0, 0));
    assert_eq!(node_ids(&normal_l0_b), (0, 1));
    assert_eq!(node_ids(&label_l1), (-1, -1));
    // Layer 1 has no normal nodes, so this node should be on layer id 1.
    assert_eq!(node_ids(&normal_l2), (1, 0));
}
