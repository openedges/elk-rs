use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LLabel, LNode, LNodeRef, LPort, LPortRef, Layer, NodeType,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::LabelDummyRemover;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions, Origin,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::edge_routing::EdgeRouting;
use org_eclipse_elk_core::org::eclipse::elk::core::options::label_side::LabelSide;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_single_layer() -> (LGraphRef, Arc<Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>, node_type: NodeType) -> LNodeRef {
    let node = LNode::new(graph);
    {
        let mut node_guard = node.lock();        node_guard.set_node_type(node_type);
    }
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock();        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef) -> LEdgeRef {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
    edge
}

#[test]
fn test_removed_nodes() {
    let (graph, layer) = graph_with_single_layer();
    {
        let mut graph_guard = graph.lock();        graph_guard.set_property(LayeredOptions::DIRECTION, Some(Direction::Right));
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(LayeredOptions::SPACING_EDGE_LABEL, Some(2.0));
        graph_guard.set_property(LayeredOptions::SPACING_LABEL_LABEL, Some(1.0));
    }

    let source = add_node(&graph, &layer, NodeType::Normal);
    let dummy = add_node(&graph, &layer, NodeType::Label);
    let target = add_node(&graph, &layer, NodeType::Normal);

    {
        let mut dummy_guard = dummy.lock();        dummy_guard.shape().position().x = 10.0;
        dummy_guard.shape().position().y = 20.0;
        dummy_guard.shape().size().x = 60.0;
        dummy_guard.shape().size().y = 30.0;
    }

    let source_port = add_port(&source, PortSide::East);
    let dummy_west = add_port(&dummy, PortSide::West);
    let dummy_east = add_port(&dummy, PortSide::East);
    let target_port = add_port(&target, PortSide::West);

    let surviving_edge = connect(&source_port, &dummy_west);
    let _dropped_edge = connect(&dummy_east, &target_port);

    let represented_label = Arc::new(Mutex::new(LLabel::with_text("edge-label")));
    {
        let mut label_guard = represented_label.lock();        label_guard.shape().size().x = 20.0;
        label_guard.shape().size().y = 6.0;
    }
    {
        let mut dummy_guard = dummy.lock();        dummy_guard.set_property(
            InternalProperties::ORIGIN,
            Some(Origin::LEdge(surviving_edge.clone())),
        );
        dummy_guard.set_property(
            InternalProperties::REPRESENTED_LABELS,
            Some(vec![represented_label.clone()]),
        );
        dummy_guard.set_property(InternalProperties::LABEL_SIDE, Some(LabelSide::Below));
    }

    let mut remover = LabelDummyRemover;
    let mut monitor = NullElkProgressMonitor;
    remover.process(&mut graph.lock(), &mut monitor);

    let nodes = layer.lock().nodes().clone();
    assert!(
        nodes.iter().all(|node| {
            node.lock().node_type() != NodeType::Label
        }),
        "all label dummy nodes must be removed"
    );

    let has_label = surviving_edge
        .lock()
        
        .labels()
        .iter()
        .any(|label| Arc::ptr_eq(label, &represented_label));
    assert!(
        has_label,
        "represented labels must be moved back to the origin edge"
    );

    let target_after_join = surviving_edge
        .lock()
        
        .target()
        .expect("edge target after join");
    assert!(
        Arc::ptr_eq(&target_after_join, &target_port),
        "joined edge must target the original downstream port"
    );

    let label_y = represented_label
        .lock()
        
        .shape()
        .position_ref()
        .y;
    assert!(
        label_y > 20.0,
        "label position must be updated from dummy placement"
    );
}

#[test]
fn test_vertical_up_reverses_label_iteration_order() {
    let (graph, layer) = graph_with_single_layer();
    {
        let mut graph_guard = graph.lock();        graph_guard.set_property(LayeredOptions::DIRECTION, Some(Direction::Up));
        graph_guard.set_property(LayeredOptions::EDGE_ROUTING, Some(EdgeRouting::Orthogonal));
        graph_guard.set_property(LayeredOptions::SPACING_EDGE_LABEL, Some(2.0));
        graph_guard.set_property(LayeredOptions::SPACING_LABEL_LABEL, Some(1.0));
    }

    let source = add_node(&graph, &layer, NodeType::Normal);
    let dummy = add_node(&graph, &layer, NodeType::Label);
    let target = add_node(&graph, &layer, NodeType::Normal);

    {
        let mut dummy_guard = dummy.lock();        dummy_guard.shape().position().x = 5.0;
        dummy_guard.shape().position().y = 7.0;
        dummy_guard.shape().size().x = 40.0;
        dummy_guard.shape().size().y = 20.0;
    }

    let source_port = add_port(&source, PortSide::East);
    let dummy_west = add_port(&dummy, PortSide::West);
    let dummy_east = add_port(&dummy, PortSide::East);
    let target_port = add_port(&target, PortSide::West);

    let origin_edge = connect(&source_port, &dummy_west);
    let _next_edge = connect(&dummy_east, &target_port);

    let label_a = Arc::new(Mutex::new(LLabel::with_text("a")));
    let label_b = Arc::new(Mutex::new(LLabel::with_text("b")));
    {
        let mut label_a_guard = label_a.lock();        label_a_guard.shape().size().x = 10.0;
        label_a_guard.shape().size().y = 6.0;
    }
    {
        let mut label_b_guard = label_b.lock();        label_b_guard.shape().size().x = 10.0;
        label_b_guard.shape().size().y = 6.0;
    }

    {
        let mut dummy_guard = dummy.lock();        dummy_guard.set_property(InternalProperties::ORIGIN, Some(Origin::LEdge(origin_edge)));
        dummy_guard.set_property(
            InternalProperties::REPRESENTED_LABELS,
            Some(vec![label_a.clone(), label_b.clone()]),
        );
        dummy_guard.set_property(InternalProperties::LABEL_SIDE, Some(LabelSide::Above));
    }

    let mut remover = LabelDummyRemover;
    let mut monitor = NullElkProgressMonitor;
    remover.process(&mut graph.lock(), &mut monitor);

    let label_a_x = label_a
        .lock()
        
        .shape()
        .position_ref()
        .x;
    let label_b_x = label_b
        .lock()
        
        .shape()
        .position_ref()
        .x;

    assert!(
        label_b_x < label_a_x,
        "UP direction must place labels in reverse iteration order"
    );
}
