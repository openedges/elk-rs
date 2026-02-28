
use std::sync::Arc;

use crate::common::issue_support::init_layered_options;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    BreakingPointInserter, BreakingPointProcessor, BreakingPointRemover,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    CuttingStrategy, InternalProperties, LayeredOptions, ValidifyStrategy,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
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

fn connect(source_node: &LNodeRef, target_node: &LNodeRef) -> LEdgeRef {
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
    edge
}

fn count_nodes_by_type(graph: &LGraphRef, node_type: NodeType) -> usize {
    graph
        .lock()
        .ok()
        .map(|graph_guard| {
            graph_guard
                .layers()
                .iter()
                .map(|layer| {
                    layer
                        .lock()
                        .ok()
                        .map(|layer_guard| {
                            layer_guard
                                .nodes()
                                .iter()
                                .filter(|node| {
                                    node.lock()
                                        .ok()
                                        .map(|node_guard| node_guard.node_type() == node_type)
                                        .unwrap_or(false)
                                })
                                .count()
                        })
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0)
}

fn setup_multi_edge_graph() -> (LGraphRef, LNodeRef, LNodeRef, LEdgeRef) {
    let (graph, layers) = new_graph_with_layers(4);
    let source = add_node(&graph, &layers[0]);
    let _mid_left = add_node(&graph, &layers[1]);
    let _mid_right = add_node(&graph, &layers[2]);
    let target = add_node(&graph, &layers[3]);

    let edge = connect(&source, &target);

    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::WRAPPING_CUTTING_STRATEGY,
            Some(CuttingStrategy::Manual),
        );
        graph_guard.set_property(LayeredOptions::WRAPPING_CUTTING_CUTS, Some(vec![1]));
        graph_guard.set_property(
            LayeredOptions::WRAPPING_VALIDIFY_STRATEGY,
            Some(ValidifyStrategy::No),
        );
    }

    (graph, source, target, edge)
}

#[test]
fn breaking_point_inserter_splits_edge_and_creates_breaking_points() {
    init_layered_options();

    let (graph, _source, _target, edge) = setup_multi_edge_graph();

    let mut inserter = BreakingPointInserter;
    let mut monitor = NullElkProgressMonitor;
    if let Ok(mut graph_guard) = graph.lock() {
        inserter.process(&mut graph_guard, &mut monitor);
    }

    let layer_count = graph
        .lock()
        .ok()
        .map(|graph_guard| graph_guard.layers().len())
        .unwrap_or(0);
    assert_eq!(layer_count, 6);

    assert_eq!(count_nodes_by_type(&graph, NodeType::BreakingPoint), 2);

    let source_node = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.source())
        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
        .expect("edge source after splitting");
    let source_node_type = source_node
        .lock()
        .ok()
        .map(|node_guard| node_guard.node_type())
        .unwrap_or(NodeType::Normal);
    assert_eq!(source_node_type, NodeType::BreakingPoint);

    let bp_info_count = graph
        .lock()
        .ok()
        .map(|graph_guard| {
            graph_guard
                .layers()
                .iter()
                .flat_map(|layer| {
                    layer
                        .lock()
                        .ok()
                        .map(|layer_guard| layer_guard.nodes().clone())
                        .unwrap_or_default()
                })
                .filter(|node| {
                    node.lock()
                        .ok()
                        .and_then(|mut node_guard| {
                            node_guard.get_property(InternalProperties::BREAKING_POINT_INFO)
                        })
                        .is_some()
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(bp_info_count, 2);
}

#[test]
fn breaking_point_processor_wraps_layers_and_marks_graph_cyclic() {
    init_layered_options();

    let (graph, _source, _target, _edge) = setup_multi_edge_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(
            LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_WRAPPED_EDGES,
            Some(false),
        );
    }

    let mut inserter = BreakingPointInserter;
    let mut processor = BreakingPointProcessor;
    let mut monitor = NullElkProgressMonitor;
    if let Ok(mut graph_guard) = graph.lock() {
        inserter.process(&mut graph_guard, &mut monitor);
        processor.process(&mut graph_guard, &mut monitor);
    }

    let cyclic = graph
        .lock()
        .ok()
        .and_then(|mut graph_guard| graph_guard.get_property(InternalProperties::CYCLIC))
        .unwrap_or(false);
    assert!(cyclic);

    assert!(count_nodes_by_type(&graph, NodeType::LongEdge) > 0);

    let has_empty_layer = graph
        .lock()
        .ok()
        .map(|graph_guard| {
            graph_guard.layers().iter().any(|layer| {
                layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(!has_empty_layer);
}

#[test]
fn breaking_point_remover_restores_original_edge_and_removes_dummies() {
    init_layered_options();

    let (graph, source, _target, edge) = setup_multi_edge_graph();
    if let Ok(mut graph_guard) = graph.lock() {
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Polyline));
    }

    let mut inserter = BreakingPointInserter;
    let mut remover = BreakingPointRemover;
    let mut monitor = NullElkProgressMonitor;
    if let Ok(mut graph_guard) = graph.lock() {
        inserter.process(&mut graph_guard, &mut monitor);
        remover.process(&mut graph_guard, &mut monitor);
    }

    assert_eq!(count_nodes_by_type(&graph, NodeType::BreakingPoint), 0);

    let restored_source = edge
        .lock()
        .ok()
        .and_then(|edge_guard| edge_guard.source())
        .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
        .expect("restored source");
    assert!(Arc::ptr_eq(&restored_source, &source));
}
