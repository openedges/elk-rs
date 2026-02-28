use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef, Layer, LayerRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::{
    IntermediateProcessorStrategy, PortListSorter,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor_factory::ILayoutProcessorFactory;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn graph_with_one_node() -> (LGraphRef, LayerRef, LNodeRef) {
    let graph = LGraph::new();
    let layer = Layer::new(&graph);
    {
        let mut graph_guard = graph.lock().expect("graph lock");
        graph_guard.layers_mut().push(layer.clone());
    }
    let node = LNode::new(&graph);
    LNode::set_layer(&node, Some(layer.clone()));
    (graph, layer, node)
}

fn add_port(node: &LNodeRef, side: PortSide, index: Option<i32>, x: f64, y: f64) -> LPortRef {
    let port = LPort::new();
    {
        let mut port_guard = port.lock().expect("port lock");
        port_guard.set_side(side);
        port_guard.shape().position().x = x;
        port_guard.shape().position().y = y;
        if let Some(idx) = index {
            port_guard.set_property(LayeredOptions::PORT_INDEX, Some(idx));
        }
    }
    LPort::set_node(&port, Some(node.clone()));
    port
}

fn run_sorter_direct(graph: &LGraphRef) {
    let mut sorter = PortListSorter;
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock().expect("graph lock");
    sorter.process(&mut graph_guard, &mut monitor);
}

fn run_sorter_via_strategy(graph: &LGraphRef) {
    let mut processor = IntermediateProcessorStrategy::PortListSorter.create();
    let mut monitor = NullElkProgressMonitor;
    let mut graph_guard = graph.lock().expect("graph lock");
    processor.process(&mut graph_guard, &mut monitor);
}

fn current_ports(node: &LNodeRef) -> Vec<LPortRef> {
    node.lock().expect("node lock").ports().clone()
}

#[test]
fn port_list_sorter_orders_by_side_and_index_for_fixed_order() {
    let (graph, _, node) = graph_with_one_node();
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedOrder),
        );
    }

    let n2 = add_port(&node, PortSide::North, Some(2), 50.0, 0.0);
    let w1 = add_port(&node, PortSide::West, Some(1), 0.0, 10.0);
    let e1 = add_port(&node, PortSide::East, Some(1), 0.0, 20.0);
    let s2 = add_port(&node, PortSide::South, Some(2), 80.0, 0.0);
    let n1 = add_port(&node, PortSide::North, Some(1), 10.0, 0.0);
    let e2 = add_port(&node, PortSide::East, Some(2), 0.0, 30.0);
    let s1 = add_port(&node, PortSide::South, Some(1), 20.0, 0.0);
    let w2 = add_port(&node, PortSide::West, Some(2), 0.0, 40.0);

    run_sorter_via_strategy(&graph);

    let ports = current_ports(&node);
    assert!(Arc::ptr_eq(&ports[0], &n1));
    assert!(Arc::ptr_eq(&ports[1], &n2));
    assert!(Arc::ptr_eq(&ports[2], &e1));
    assert!(Arc::ptr_eq(&ports[3], &e2));
    assert!(Arc::ptr_eq(&ports[4], &s1));
    assert!(Arc::ptr_eq(&ports[5], &s2));
    assert!(Arc::ptr_eq(&ports[6], &w1));
    assert!(Arc::ptr_eq(&ports[7], &w2));
}

#[test]
fn port_list_sorter_orders_fixed_pos_ports_clockwise_by_position() {
    let (graph, _, node) = graph_with_one_node();
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedPos),
        );
    }

    let n_hi = add_port(&node, PortSide::North, None, 30.0, 0.0);
    let s_left = add_port(&node, PortSide::South, None, 10.0, 0.0);
    let w_hi = add_port(&node, PortSide::West, None, 0.0, 30.0);
    let e_lo = add_port(&node, PortSide::East, None, 0.0, 10.0);
    let n_lo = add_port(&node, PortSide::North, None, 10.0, 0.0);
    let w_lo = add_port(&node, PortSide::West, None, 0.0, 10.0);
    let s_right = add_port(&node, PortSide::South, None, 30.0, 0.0);
    let e_hi = add_port(&node, PortSide::East, None, 0.0, 30.0);

    run_sorter_direct(&graph);

    let ports = current_ports(&node);
    assert!(Arc::ptr_eq(&ports[0], &n_lo));
    assert!(Arc::ptr_eq(&ports[1], &n_hi));
    assert!(Arc::ptr_eq(&ports[2], &e_lo));
    assert!(Arc::ptr_eq(&ports[3], &e_hi));
    assert!(Arc::ptr_eq(&ports[4], &s_right));
    assert!(Arc::ptr_eq(&ports[5], &s_left));
    assert!(Arc::ptr_eq(&ports[6], &w_hi));
    assert!(Arc::ptr_eq(&ports[7], &w_lo));
}

#[test]
fn port_list_sorter_skips_nodes_with_free_constraints() {
    let (graph, _, node) = graph_with_one_node();
    {
        let mut node_guard = node.lock().expect("node lock");
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::Free),
        );
    }

    let p1 = add_port(&node, PortSide::West, Some(2), 0.0, 10.0);
    let p2 = add_port(&node, PortSide::North, Some(1), 5.0, 0.0);
    let before = current_ports(&node);
    assert!(Arc::ptr_eq(&before[0], &p1));
    assert!(Arc::ptr_eq(&before[1], &p2));

    run_sorter_direct(&graph);

    let after = current_ports(&node);
    assert!(Arc::ptr_eq(&after[0], &p1));
    assert!(Arc::ptr_eq(&after[1], &p2));
}
