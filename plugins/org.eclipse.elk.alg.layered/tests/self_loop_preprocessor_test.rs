use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::SelfLoopPreProcessor;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::NullElkProgressMonitor;

fn node(graph: &LGraphRef) -> LNodeRef {
    let lnode = LNode::new(graph);
    graph
        .lock()
        .expect("graph lock")
        .layerless_nodes_mut()
        .push(lnode.clone());
    lnode
}

fn edge(source: &LPortRef, target: &LPortRef) -> LEdgeRef {
    let ledge = LEdge::new();
    LEdge::set_source(&ledge, Some(source.clone()));
    LEdge::set_target(&ledge, Some(target.clone()));
    ledge
}

fn ports(node: &LNodeRef, north: usize, east: usize, south: usize, west: usize) {
    for _ in 0..north {
        let port = LPort::new();
        port.lock().expect("port lock").set_side(PortSide::North);
        LPort::set_node(&port, Some(node.clone()));
    }
    for _ in 0..east {
        let port = LPort::new();
        port.lock().expect("port lock").set_side(PortSide::East);
        LPort::set_node(&port, Some(node.clone()));
    }
    for _ in 0..south {
        let port = LPort::new();
        port.lock().expect("port lock").set_side(PortSide::South);
        LPort::set_node(&port, Some(node.clone()));
    }
    for _ in 0..west {
        let port = LPort::new();
        port.lock().expect("port lock").set_side(PortSide::West);
        LPort::set_node(&port, Some(node.clone()));
    }
}

fn basic_graph_without_self_loops() -> LGraphRef {
    let graph = LGraph::new();

    let n1 = node(&graph);
    ports(&n1, 0, 1, 0, 0);

    let n2 = node(&graph);
    ports(&n2, 0, 0, 0, 1);

    let n1_port = n1.lock().expect("n1 lock").ports()[0].clone();
    let n2_port = n2.lock().expect("n2 lock").ports()[0].clone();
    let _ = edge(&n1_port, &n2_port);

    graph
}

fn contains_port(ports: &[LPortRef], candidate: &LPortRef) -> bool {
    ports.iter().any(|port| Arc::ptr_eq(port, candidate))
}

struct TestGraph {
    graph: LGraphRef,
    self_loop_node: LNodeRef,
    regular_ports: Vec<LPortRef>,
    only_self_loop_ports: Vec<LPortRef>,
    unconnected_ports: Vec<LPortRef>,
}

impl TestGraph {
    fn new(port_constraints: PortConstraints) -> Self {
        let graph = basic_graph_without_self_loops();
        let self_loop_node = graph.lock().expect("graph lock").layerless_nodes()[1].clone();
        {
            let mut node_guard = self_loop_node.lock().expect("self loop node lock");
            node_guard.set_property(LayeredOptions::PORT_CONSTRAINTS, Some(port_constraints));
            node_guard.shape().size().x = 50.0;
            node_guard.shape().size().y = 50.0;
        }

        ports(&self_loop_node, 2, 2, 2, 1);

        let (node_width, node_height, all_ports) = {
            let mut node_guard = self_loop_node.lock().expect("self loop node lock");
            let width = node_guard.shape().size_ref().x;
            let height = node_guard.shape().size_ref().y;
            let ports = node_guard.ports().clone();
            (width, height, ports)
        };

        let mut north = 10.0;
        let mut east = 10.0;
        let mut south = 10.0;
        let mut west = 10.0;
        for port in &all_ports {
            let side = port.lock().expect("port lock").side();
            let mut port_guard = port.lock().expect("port lock");
            match side {
                PortSide::North => {
                    port_guard.shape().position().x = north;
                    port_guard.shape().position().y = 0.0;
                    north += 30.0;
                }
                PortSide::East => {
                    port_guard.shape().position().x = node_width;
                    port_guard.shape().position().y = east;
                    east += 30.0;
                }
                PortSide::South => {
                    port_guard.shape().position().x = south;
                    port_guard.shape().position().y = node_height;
                    south += 30.0;
                }
                PortSide::West => {
                    port_guard.shape().position().x = 0.0;
                    port_guard.shape().position().y = west;
                    west += 30.0;
                }
                PortSide::Undefined => panic!("unexpected undefined port side"),
            }
        }

        let ports = self_loop_node.lock().expect("self loop node lock").ports().clone();
        assert_eq!(8, ports.len());

        let _ = edge(&ports[0], &ports[1]);
        let _ = edge(&ports[1], &ports[2]);
        let _ = edge(&ports[3], &ports[4]);
        let _ = edge(&ports[4], &ports[5]);

        TestGraph {
            graph,
            self_loop_node,
            regular_ports: vec![ports[0].clone()],
            only_self_loop_ports: vec![
                ports[1].clone(),
                ports[2].clone(),
                ports[3].clone(),
                ports[4].clone(),
                ports[5].clone(),
            ],
            unconnected_ports: vec![ports[6].clone(), ports[7].clone()],
        }
    }
}

fn run_preprocessor(graph: &LGraphRef) {
    let mut preprocessor = SelfLoopPreProcessor;
    let mut monitor = NullElkProgressMonitor;
    preprocessor.process(&mut graph.lock().expect("graph lock"), &mut monitor);
}

fn test_hidden_edges(test_graph: &TestGraph) {
    let connected_edges = test_graph
        .self_loop_node
        .lock()
        .expect("self loop node lock")
        .connected_edges();

    for ledge in &connected_edges {
        assert!(!ledge.lock().expect("edge lock").is_self_loop());
    }
    assert!(!connected_edges.is_empty());
}

fn test_ports_hidden(port_constraints: PortConstraints) {
    let test_graph = TestGraph::new(port_constraints);
    run_preprocessor(&test_graph.graph);

    let remaining_ports = test_graph
        .self_loop_node
        .lock()
        .expect("self loop node lock")
        .ports()
        .clone();
    assert_eq!(
        test_graph.regular_ports.len() + test_graph.unconnected_ports.len(),
        remaining_ports.len()
    );
    for port in &test_graph.regular_ports {
        assert!(contains_port(&remaining_ports, port));
    }
    for port in &test_graph.unconnected_ports {
        assert!(contains_port(&remaining_ports, port));
    }
    for port in &test_graph.only_self_loop_ports {
        assert!(!contains_port(&remaining_ports, port));
    }

    test_hidden_edges(&test_graph);
}

fn test_ports_untouched(port_constraints: PortConstraints) {
    let test_graph = TestGraph::new(port_constraints);
    run_preprocessor(&test_graph.graph);

    let remaining_ports = test_graph
        .self_loop_node
        .lock()
        .expect("self loop node lock")
        .ports()
        .clone();
    assert_eq!(
        test_graph.regular_ports.len()
            + test_graph.only_self_loop_ports.len()
            + test_graph.unconnected_ports.len(),
        remaining_ports.len()
    );

    test_hidden_edges(&test_graph);
}

#[test]
fn test_free_ports() {
    test_ports_hidden(PortConstraints::Free);
}

#[test]
fn test_fixed_side() {
    test_ports_hidden(PortConstraints::FixedSide);
}

#[test]
fn test_fixed_order() {
    test_ports_untouched(PortConstraints::FixedOrder);
}

#[test]
fn test_fixed_ratio() {
    test_ports_untouched(PortConstraints::FixedRatio);
}

#[test]
fn test_fixed_pos() {
    test_ports_untouched(PortConstraints::FixedPos);
}
