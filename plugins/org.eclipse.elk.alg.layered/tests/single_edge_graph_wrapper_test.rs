mod issue_support;

use issue_support::init_layered_options;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::single_edge_graph_wrapper::GraphStats;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::SingleEdgeGraphWrapper;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CuttingStrategy, InternalProperties, LayeredOptions, ValidifyStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn new_graph_with_layers(count: usize) -> (LGraphRef, Vec<LayerRef>) {
    let graph = LGraph::new();
    let mut layers = Vec::with_capacity(count);
    if let Ok(mut graph_guard) = graph.lock() {
        for _ in 0..count {
            let layer = Layer::new(&graph);
            graph_guard.layers_mut().push(layer.clone());
            layers.push(layer);
        }
    }
    (graph, layers)
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn connect(source_node: &LNodeRef, target_node: &LNodeRef) {
    let source_port = LPort::new();
    if let Ok(mut source_guard) = source_port.lock() {
        source_guard.set_side(PortSide::East);
    }
    LPort::set_node(&source_port, Some(source_node.clone()));

    let target_port = LPort::new();
    if let Ok(mut target_guard) = target_port.lock() {
        target_guard.set_side(PortSide::West);
    }
    LPort::set_node(&target_port, Some(target_node.clone()));

    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source_port));
    LEdge::set_target(&edge, Some(target_port));
}

fn build_path_graph(layer_count: usize) -> LGraphRef {
    let (graph, layers) = new_graph_with_layers(layer_count);
    let mut nodes = Vec::new();
    for layer in &layers {
        nodes.push(add_node(&graph, layer));
    }
    for pair in nodes.windows(2) {
        connect(&pair[0], &pair[1]);
    }
    graph
}

#[test]
fn validify_indexes_greedily_keeps_valid_cuts_for_simple_path() {
    init_layered_options();

    let graph = build_path_graph(5);
    let graph_stats = graph
        .lock()
        .ok()
        .map(|mut graph_guard| GraphStats::new(&mut graph_guard))
        .expect("graph stats");

    let desired = vec![1, 3];
    let valid = SingleEdgeGraphWrapper::validify_indexes_greedily(&graph_stats, desired.clone());
    assert_eq!(valid, desired);
}

#[test]
fn manual_cut_process_marks_graph_as_cyclic() {
    init_layered_options();

    let graph = build_path_graph(5);
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::WRAPPING_CUTTING_STRATEGY,
            Some(CuttingStrategy::Manual),
        );
        graph_guard.set_property(LayeredOptions::WRAPPING_CUTTING_CUTS, Some(vec![2]));
        graph_guard.set_property(
            LayeredOptions::WRAPPING_VALIDIFY_STRATEGY,
            Some(ValidifyStrategy::No),
        );
    }

    let mut wrapper = SingleEdgeGraphWrapper;
    let mut monitor = NullElkProgressMonitor;
    if let Ok(mut graph_guard) = graph.lock() {
        wrapper.process(&mut graph_guard, &mut monitor);
    }

    let cyclic = graph
        .lock()
        .ok()
        .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::CYCLIC))
        .unwrap_or(false);
    assert!(cyclic);
}
