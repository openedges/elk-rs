use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::CommentPostprocessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

const EPS: f64 = 1e-6;

fn graph_with_layer() -> (LGraphRef, LayerRef) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &LayerRef) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_comment_box(graph: &LGraphRef, w: f64, h: f64, side: PortSide) -> (LNodeRef, LPortRef) {
    let box_node = LNode::new(graph);
    {
        let mut box_guard = box_node.lock();        box_guard.shape().size().x = w;
        box_guard.shape().size().y = h;
    }
    let box_port = LPort::new();
    {
        let mut box_port_guard = box_port.lock();        box_port_guard.set_side(side);
    }
    LPort::set_node(&box_port, Some(box_node.clone()));
    (box_node, box_port)
}

fn run_processor(graph: &LGraphRef) {
    let mut processor = CommentPostprocessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock();    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn comment_postprocessor_reinserts_boxes_and_reconnects_edges() {
    let (graph, layer) = graph_with_layer();
    graph
        .lock()
        
        .set_property(LayeredOptions::SPACING_COMMENT_COMMENT, Some(5.0));

    let node = add_node(&graph, &layer);
    {
        let mut node_guard = node.lock();        node_guard.shape().position().x = 100.0;
        node_guard.shape().position().y = 100.0;
        node_guard.shape().size().x = 60.0;
        node_guard.shape().size().y = 40.0;
        node_guard.margin().top = 20.0;
        node_guard.margin().bottom = 30.0;
    }

    let (top_box_1, top_port_1) = add_comment_box(&graph, 20.0, 10.0, PortSide::South);
    let (top_box_2, top_port_2) = add_comment_box(&graph, 10.0, 8.0, PortSide::South);
    let (bottom_box, bottom_port) = add_comment_box(&graph, 30.0, 12.0, PortSide::North);

    let node_top_port_1 = LPort::new();
    let node_top_port_2 = LPort::new();
    let node_bottom_port = LPort::new();
    {
        top_box_1.lock().set_property(
            InternalProperties::COMMENT_CONN_PORT,
            Some(node_top_port_1.clone()),
        );
        top_box_2.lock().set_property(
            InternalProperties::COMMENT_CONN_PORT,
            Some(node_top_port_2.clone()),
        );
        bottom_box.lock().set_property(
            InternalProperties::COMMENT_CONN_PORT,
            Some(node_bottom_port.clone()),
        );
    }

    let edge_top_1 = LEdge::new();
    let edge_top_2 = LEdge::new();
    let edge_bottom = LEdge::new();
    LEdge::set_source(&edge_top_1, Some(top_port_1.clone()));
    LEdge::set_source(&edge_top_2, Some(top_port_2.clone()));
    LEdge::set_target(&edge_bottom, Some(bottom_port.clone()));

    {
        node.lock().set_property(
            InternalProperties::TOP_COMMENTS,
            Some(vec![top_box_1.clone(), top_box_2.clone()]),
        );
        node.lock().set_property(
            InternalProperties::BOTTOM_COMMENTS,
            Some(vec![bottom_box.clone()]),
        );
    }

    run_processor(&graph);

    let layer_nodes = layer.lock().nodes().clone();
    assert!(layer_nodes.iter().any(|n| Arc::ptr_eq(n, &top_box_1)));
    assert!(layer_nodes.iter().any(|n| Arc::ptr_eq(n, &top_box_2)));
    assert!(layer_nodes.iter().any(|n| Arc::ptr_eq(n, &bottom_box)));

    let top_1_pos = *top_box_1
        .lock()
        
        .shape()
        .position_ref();
    let top_2_pos = *top_box_2
        .lock()
        
        .shape()
        .position_ref();
    let bottom_pos = *bottom_box
        .lock()
        
        .shape()
        .position_ref();

    assert!((top_1_pos.x - 112.5).abs() < EPS);
    assert!((top_1_pos.y - 80.0).abs() < EPS);
    assert!((top_2_pos.x - 137.5).abs() < EPS);
    assert!((top_2_pos.y - 82.0).abs() < EPS);
    assert!((bottom_pos.x - 115.0).abs() < EPS);
    assert!((bottom_pos.y - 158.0).abs() < EPS);

    assert!(edge_top_1
        .lock()
        
        .target()
        .is_some_and(|p| Arc::ptr_eq(&p, &node_top_port_1)));
    assert!(edge_top_2
        .lock()
        
        .target()
        .is_some_and(|p| Arc::ptr_eq(&p, &node_top_port_2)));
    assert!(edge_bottom
        .lock()
        
        .source()
        .is_some_and(|p| Arc::ptr_eq(&p, &node_bottom_port)));

    assert!(node_top_port_1
        .lock()
        
        .node()
        .is_some_and(|n| Arc::ptr_eq(&n, &node)));
    assert!(node_top_port_2
        .lock()
        
        .node()
        .is_some_and(|n| Arc::ptr_eq(&n, &node)));
    assert!(node_bottom_port
        .lock()
        
        .node()
        .is_some_and(|n| Arc::ptr_eq(&n, &node)));
}

#[test]
fn comment_postprocessor_leaves_layer_unchanged_without_comment_properties() {
    let (graph, layer) = graph_with_layer();
    let node = add_node(&graph, &layer);

    run_processor(&graph);

    let layer_nodes = layer.lock().nodes().clone();
    assert_eq!(layer_nodes.len(), 1);
    assert!(Arc::ptr_eq(&layer_nodes[0], &node));
}
