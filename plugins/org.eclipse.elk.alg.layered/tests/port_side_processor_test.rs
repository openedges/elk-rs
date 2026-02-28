use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::PortSideProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::{
    InternalProperties, LayeredOptions,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_layer() -> (LGraphRef, Arc<Mutex<Layer>>) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    graph
        .lock()
        .expect("graph lock")
        .layers_mut()
        .push(layer.clone());
    (graph, layer)
}

fn add_node(graph: &LGraphRef, layer: &Arc<Mutex<Layer>>) -> LNodeRef {
    let node = LNode::new(graph);
    LNode::set_layer(&node, Some(layer.clone()));
    node
}

fn add_port(node: &LNodeRef, side: PortSide) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock().expect("port lock");
        port_guard.set_side(side);
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn connect(source: &LPortRef, target: &LPortRef) {
    let edge = LEdge::new();
    LEdge::set_source(&edge, Some(source.clone()));
    LEdge::set_target(&edge, Some(target.clone()));
}

fn run_processor(graph: &LGraphRef) {
    let mut processor = PortSideProcessor;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock().expect("graph lock");
    processor.process(&mut graph_guard, &mut monitor);
}

#[test]
fn port_side_processor_assigns_sides_and_fixes_constraints() {
    let (graph, layer) = graph_with_layer();
    let node = add_node(&graph, &layer);
    let other = add_node(&graph, &layer);

    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::Free),
        );
    }

    let out_port = add_port(&node, PortSide::Undefined);
    let in_port = add_port(&node, PortSide::Undefined);
    let other_west = add_port(&other, PortSide::West);
    let other_east = add_port(&other, PortSide::East);
    connect(&out_port, &other_west);
    connect(&other_east, &in_port);

    run_processor(&graph);

    let constraints = node
        .lock()
        .expect("node lock")
        .get_property(LayeredOptions::PORT_CONSTRAINTS)
        .unwrap_or(PortConstraints::Undefined);
    assert_eq!(constraints, PortConstraints::FixedSide);
    assert_eq!(out_port.lock().expect("port lock").side(), PortSide::East);
    assert_eq!(in_port.lock().expect("port lock").side(), PortSide::West);
}

#[test]
fn port_side_processor_only_fills_undefined_ports_when_side_fixed() {
    let (graph, layer) = graph_with_layer();
    let node = add_node(&graph, &layer);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
    }

    let fixed = add_port(&node, PortSide::North);
    let undefined = add_port(&node, PortSide::Undefined);

    run_processor(&graph);

    assert_eq!(fixed.lock().expect("fixed lock").side(), PortSide::North);
    assert_ne!(
        undefined.lock().expect("undefined lock").side(),
        PortSide::Undefined
    );
}

#[test]
fn port_side_processor_prefers_port_dummy_external_side() {
    let (graph, layer) = graph_with_layer();
    let node = add_node(&graph, &layer);
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::Free),
        );
    }

    let dummy = add_node(&graph, &layer);
    {
        let mut dummy_guard = dummy.lock().expect("dummy lock");
        dummy_guard.set_property(InternalProperties::EXT_PORT_SIDE, Some(PortSide::South));
    }
    let port = add_port(&node, PortSide::Undefined);
    {
        let mut port_guard = port.lock().expect("port lock");
        port_guard.set_property(InternalProperties::PORT_DUMMY, Some(dummy.clone()));
    }

    run_processor(&graph);
    assert_eq!(port.lock().expect("port lock").side(), PortSide::South);
}
