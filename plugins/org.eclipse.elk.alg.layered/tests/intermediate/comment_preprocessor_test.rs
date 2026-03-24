use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::CommentPreprocessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn add_layerless_node(graph: &LGraphRef, comment_box: bool) -> LNodeRef {
    let node = LNode::new(graph);
    if comment_box {
        node.lock()
            
            .set_property(LayeredOptions::COMMENT_BOX, Some(true));
    }
    graph
        .lock()
        
        .layerless_nodes_mut()
        .push(node.clone());
    node
}

fn add_port(node: &LNodeRef) -> LPortRef {
    let port = LPort::new();
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn run_preprocessor(graph: &LGraphRef) {
    let mut processor = CommentPreprocessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn comment_preprocessor_extracts_single_connection_comment() {
    let graph = LGraph::new();
    let real_node = add_layerless_node(&graph, false);
    let comment_node = add_layerless_node(&graph, true);

    let real_port = add_port(&real_node);
    let comment_port = add_port(&comment_node);
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(comment_port.clone()));
    LEdge::set_target(&edge, Some(real_port.clone()));
    edge.lock()
        
        .bend_points()
        .add_values(10.0, 20.0);

    run_preprocessor(&graph);

    let layerless_nodes = graph.lock().layerless_nodes().clone();
    assert_eq!(layerless_nodes.len(), 1);
    assert!(Arc::ptr_eq(&layerless_nodes[0], &real_node));

    let in_top = real_node
        .lock()
        
        .get_property(InternalProperties::TOP_COMMENTS)
        .map(|nodes| nodes.iter().any(|n| Arc::ptr_eq(n, &comment_node)))
        .unwrap_or(false);
    let in_bottom = real_node
        .lock()
        
        .get_property(InternalProperties::BOTTOM_COMMENTS)
        .map(|nodes| nodes.iter().any(|n| Arc::ptr_eq(n, &comment_node)))
        .unwrap_or(false);
    assert!(in_top || in_bottom);

    assert!(comment_node
        .lock()
        
        .get_property(InternalProperties::COMMENT_CONN_PORT)
        .is_some_and(|p| Arc::ptr_eq(&p, &real_port)));
    assert!(edge.lock().target().is_none());
    assert!(edge.lock().bend_points_ref().is_empty());
}

#[test]
fn comment_preprocessor_keeps_multi_connected_comment() {
    let graph = LGraph::new();
    let real_a = add_layerless_node(&graph, false);
    let real_b = add_layerless_node(&graph, false);
    let comment_node = add_layerless_node(&graph, true);

    let real_a_port = add_port(&real_a);
    let real_b_port = add_port(&real_b);
    let comment_port_a = add_port(&comment_node);
    let comment_port_b = add_port(&comment_node);

    let edge_a = LEdge::new();
    LEdge::set_source(&edge_a, Some(comment_port_a));
    LEdge::set_target(&edge_a, Some(real_a_port));
    let edge_b = LEdge::new();
    LEdge::set_source(&edge_b, Some(comment_port_b));
    LEdge::set_target(&edge_b, Some(real_b_port));

    run_preprocessor(&graph);

    let layerless_nodes = graph.lock().layerless_nodes().clone();
    assert!(layerless_nodes
        .iter()
        .any(|n| Arc::ptr_eq(n, &comment_node)));
    assert!(comment_node
        .lock()
        
        .get_property(InternalProperties::COMMENT_CONN_PORT)
        .is_none());
}
