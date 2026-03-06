use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::grid_snap_processor::{
    GridSnapGraphSizeProcessor, GridSnapPositionProcessor, GridSnapSizeProcessor,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

fn make_graph_with_node(grid_size: f64, w: f64, h: f64, x: f64, y: f64) -> (LGraphRef, LNodeRef) {
    let graph = LGraph::new();
    graph
        .lock()
        .expect("graph lock")
        .set_property(LayeredOptions::GRID_SNAP_GRID_SIZE, Some(grid_size));

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());

    let node = LNode::new(&graph);
    {
        let mut ng = node.lock().expect("node lock");
        ng.set_node_type(NodeType::Normal);
        ng.shape().size().x = w;
        ng.shape().size().y = h;
        ng.shape().position().x = x;
        ng.shape().position().y = y;
    }
    LNode::set_layer(&node, Some(layer));

    (graph, node)
}

fn add_dummy_node(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>, x: f64, y: f64) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut ng = node.lock().expect("node lock");
        ng.set_node_type(NodeType::LongEdge);
        ng.shape().size().x = 5.0;
        ng.shape().size().y = 5.0;
        ng.shape().position().x = x;
        ng.shape().position().y = y;
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

#[test]
fn grid_snap_disabled_by_default() {
    let graph = LGraph::new();
    // No grid_size property set (defaults to 0.0)
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());

    let node = LNode::new(&graph);
    {
        let mut ng = node.lock().expect("node lock");
        ng.set_node_type(NodeType::Normal);
        ng.shape().size().x = 37.0;
        ng.shape().size().y = 53.0;
        ng.shape().position().x = 17.3;
        ng.shape().position().y = 22.7;
    }
    LNode::set_layer(&node, Some(layer));

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 37.0);
    assert_eq!(ng.shape().size_ref().y, 53.0);
    assert_eq!(ng.shape().position_ref().x, 17.3);
    assert_eq!(ng.shape().position_ref().y, 22.7);
}

#[test]
fn grid_snap_node_size_ceil() {
    let (graph, node) = make_graph_with_node(10.0, 37.0, 53.0, 0.0, 0.0);

    let mut monitor = NullElkProgressMonitor;
    let mut proc = GridSnapSizeProcessor;
    proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 40.0);
    assert_eq!(ng.shape().size_ref().y, 60.0);
}

#[test]
fn grid_snap_node_size_exact_no_change() {
    let (graph, node) = make_graph_with_node(10.0, 40.0, 60.0, 0.0, 0.0);

    let mut monitor = NullElkProgressMonitor;
    let mut proc = GridSnapSizeProcessor;
    proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 40.0);
    assert_eq!(ng.shape().size_ref().y, 60.0);
}

#[test]
fn grid_snap_node_position_round() {
    let (graph, node) = make_graph_with_node(10.0, 40.0, 60.0, 17.3, 22.7);

    let mut monitor = NullElkProgressMonitor;
    let mut proc = GridSnapPositionProcessor;
    proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().position_ref().x, 20.0);
    assert_eq!(ng.shape().position_ref().y, 20.0);
}

#[test]
fn grid_snap_position_round_midpoint() {
    // 15.0 / 10.0 = 1.5, round = 2.0 → 20.0 (Rust rounds away from zero)
    // 25.0 / 10.0 = 2.5, round = 3.0 → 30.0
    let (graph, node) = make_graph_with_node(10.0, 40.0, 60.0, 15.0, 25.0);

    let mut monitor = NullElkProgressMonitor;
    let mut proc = GridSnapPositionProcessor;
    proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().position_ref().x, 20.0);
    assert_eq!(ng.shape().position_ref().y, 30.0);
}

#[test]
fn grid_snap_skips_dummy_nodes() {
    let graph = LGraph::new();
    graph
        .lock()
        .expect("graph lock")
        .set_property(LayeredOptions::GRID_SNAP_GRID_SIZE, Some(10.0));

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());

    let dummy = add_dummy_node(&graph, &layer, 17.3, 22.7);

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = dummy.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 5.0);
    assert_eq!(ng.shape().size_ref().y, 5.0);
    assert_eq!(ng.shape().position_ref().x, 17.3);
    assert_eq!(ng.shape().position_ref().y, 22.7);
}

#[test]
fn grid_snap_negative_grid_size_ignored() {
    let (graph, node) = make_graph_with_node(-10.0, 37.0, 53.0, 17.3, 22.7);

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 37.0);
    assert_eq!(ng.shape().size_ref().y, 53.0);
    assert_eq!(ng.shape().position_ref().x, 17.3);
    assert_eq!(ng.shape().position_ref().y, 22.7);
}

#[test]
fn grid_snap_zero_grid_size_ignored() {
    let (graph, node) = make_graph_with_node(0.0, 37.0, 53.0, 17.3, 22.7);

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 37.0);
    assert_eq!(ng.shape().size_ref().y, 53.0);
    assert_eq!(ng.shape().position_ref().x, 17.3);
    assert_eq!(ng.shape().position_ref().y, 22.7);
}

#[test]
fn grid_snap_multiple_nodes_multiple_layers() {
    let graph = LGraph::new();
    graph
        .lock()
        .expect("graph lock")
        .set_property(LayeredOptions::GRID_SNAP_GRID_SIZE, Some(10.0));

    let layer1 = Layer::new(&graph);
    let layer2 = Layer::new(&graph);
    {
        let mut gg = graph.lock().expect("graph lock");
        gg.layers_mut().push(layer1.clone());
        gg.layers_mut().push(layer2.clone());
    }

    let n1 = LNode::new(&graph);
    {
        let mut ng = n1.lock().expect("node lock");
        ng.set_node_type(NodeType::Normal);
        ng.shape().size().x = 33.0;
        ng.shape().size().y = 47.0;
        ng.shape().position().x = 12.3;
        ng.shape().position().y = 8.7;
    }
    LNode::set_layer(&n1, Some(layer1.clone()));

    let n2 = LNode::new(&graph);
    {
        let mut ng = n2.lock().expect("node lock");
        ng.set_node_type(NodeType::Normal);
        ng.shape().size().x = 51.0;
        ng.shape().size().y = 19.0;
        ng.shape().position().x = 100.4;
        ng.shape().position().y = 55.6;
    }
    LNode::set_layer(&n2, Some(layer2.clone()));

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng1 = n1.lock().expect("node lock");
    assert_eq!(ng1.shape().size_ref().x, 40.0);
    assert_eq!(ng1.shape().size_ref().y, 50.0);
    assert_eq!(ng1.shape().position_ref().x, 10.0);
    assert_eq!(ng1.shape().position_ref().y, 10.0);

    drop(ng1);

    let mut ng2 = n2.lock().expect("node lock");
    assert_eq!(ng2.shape().size_ref().x, 60.0);
    assert_eq!(ng2.shape().size_ref().y, 20.0);
    assert_eq!(ng2.shape().position_ref().x, 100.0);
    assert_eq!(ng2.shape().position_ref().y, 60.0);
}

#[test]
fn grid_snap_mixed_normal_and_dummy_in_layer() {
    let graph = LGraph::new();
    graph
        .lock()
        .expect("graph lock")
        .set_property(LayeredOptions::GRID_SNAP_GRID_SIZE, Some(10.0));

    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());

    let normal = LNode::new(&graph);
    {
        let mut ng = normal.lock().expect("node lock");
        ng.set_node_type(NodeType::Normal);
        ng.shape().size().x = 33.0;
        ng.shape().size().y = 47.0;
        ng.shape().position().x = 12.3;
        ng.shape().position().y = 8.7;
    }
    LNode::set_layer(&normal, Some(layer.clone()));

    let dummy = add_dummy_node(&graph, &layer, 50.5, 30.3);

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    // Normal node: snapped
    let mut ng = normal.lock().expect("node lock");
    assert_eq!(ng.shape().size_ref().x, 40.0);
    assert_eq!(ng.shape().size_ref().y, 50.0);
    assert_eq!(ng.shape().position_ref().x, 10.0);
    assert_eq!(ng.shape().position_ref().y, 10.0);
    drop(ng);

    // Dummy node: unchanged
    let mut dg = dummy.lock().expect("node lock");
    assert_eq!(dg.shape().size_ref().x, 5.0);
    assert_eq!(dg.shape().size_ref().y, 5.0);
    assert_eq!(dg.shape().position_ref().x, 50.5);
    assert_eq!(dg.shape().position_ref().y, 30.3);
}

#[test]
fn grid_snap_negative_positions() {
    let (graph, node) = make_graph_with_node(10.0, 40.0, 60.0, -17.3, -22.7);

    let mut monitor = NullElkProgressMonitor;
    let mut proc = GridSnapPositionProcessor;
    proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    assert_eq!(ng.shape().position_ref().x, -20.0);
    assert_eq!(ng.shape().position_ref().y, -20.0);
}

#[test]
fn grid_snap_non_integer_grid_size() {
    let (graph, node) = make_graph_with_node(7.5, 10.0, 20.0, 11.0, 19.0);

    let mut monitor = NullElkProgressMonitor;

    let mut size_proc = GridSnapSizeProcessor;
    size_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut pos_proc = GridSnapPositionProcessor;
    pos_proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let mut ng = node.lock().expect("node lock");
    // size: ceil(10.0/7.5)*7.5 = ceil(1.333)*7.5 = 2*7.5 = 15.0
    assert_eq!(ng.shape().size_ref().x, 15.0);
    // size: ceil(20.0/7.5)*7.5 = ceil(2.666)*7.5 = 3*7.5 = 22.5
    assert_eq!(ng.shape().size_ref().y, 22.5);
    // pos: round(11.0/7.5)*7.5 = round(1.466)*7.5 = 1*7.5 = 7.5
    assert_eq!(ng.shape().position_ref().x, 7.5);
    // pos: round(19.0/7.5)*7.5 = round(2.533)*7.5 = 3*7.5 = 22.5
    assert_eq!(ng.shape().position_ref().y, 22.5);
}

#[test]
fn grid_snap_graph_size_and_offset() {
    let graph = LGraph::new();
    {
        let mut gg = graph.lock().expect("graph lock");
        gg.set_property(LayeredOptions::GRID_SNAP_GRID_SIZE, Some(10.0));
        gg.size().x = 137.5;
        gg.size().y = 253.2;
        gg.offset().x = 12.3;
        gg.offset().y = 7.8;
    }

    let mut monitor = NullElkProgressMonitor;
    let mut proc = GridSnapGraphSizeProcessor;
    proc.process(&mut graph.lock().expect("graph lock"), &mut monitor);

    let gg = graph.lock().expect("graph lock");
    // size: ceil snap
    assert_eq!(gg.size_ref().x, 140.0);
    assert_eq!(gg.size_ref().y, 260.0);
    // offset: round snap
    assert_eq!(gg.offset_ref().x, 10.0);
    assert_eq!(gg.offset_ref().y, 10.0);
}
