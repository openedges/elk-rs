use std::collections::BTreeSet;
use std::sync::Arc;

use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LPortRef,
};
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopHolder;
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::InternalProperties;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;

fn node(graph: &LGraphRef) -> LNodeRef {
    let lnode = LNode::new(graph);
    graph
        .lock()
        
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
        port.lock().set_side(PortSide::North);
        LPort::set_node(&port, Some(node.clone()));
    }
    for _ in 0..east {
        let port = LPort::new();
        port.lock().set_side(PortSide::East);
        LPort::set_node(&port, Some(node.clone()));
    }
    for _ in 0..south {
        let port = LPort::new();
        port.lock().set_side(PortSide::South);
        LPort::set_node(&port, Some(node.clone()));
    }
    for _ in 0..west {
        let port = LPort::new();
        port.lock().set_side(PortSide::West);
        LPort::set_node(&port, Some(node.clone()));
    }
}

fn basic_graph_without_self_loops() -> LGraphRef {
    let graph = LGraph::new();

    let n1 = node(&graph);
    ports(&n1, 0, 1, 0, 0);

    let n2 = node(&graph);
    ports(&n2, 0, 0, 0, 1);

    let n1_port = n1.lock().ports()[0].clone();
    let n2_port = n2.lock().ports()[0].clone();
    let _ = edge(&n1_port, &n2_port);

    graph
}

fn basic_graph_with_self_loops() -> LGraphRef {
    let graph = basic_graph_without_self_loops();

    let lnode = graph.lock().layerless_nodes()[1].clone();
    ports(&lnode, 2, 2, 2, 1);

    let port_list = lnode.lock().ports().clone();
    for i in (1..port_list.len()).step_by(2) {
        let _ = edge(&port_list[i - 1], &port_list[i]);
    }

    graph
}

fn edge_key(edge: &LEdgeRef) -> usize {
    Arc::as_ptr(edge) as usize
}

#[test]
fn test_non_self_loop_graph() {
    let graph = basic_graph_without_self_loops();
    for lnode in graph.lock().layerless_nodes().clone() {
        assert!(!SelfLoopHolder::needs_self_loop_processing(&lnode));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = SelfLoopHolder::install(&lnode);
        }));
        assert!(result.is_err());
    }
}

#[test]
fn test_basic_self_loops() {
    let graph = basic_graph_with_self_loops();
    let lnode = graph.lock().layerless_nodes()[1].clone();

    assert!(SelfLoopHolder::needs_self_loop_processing(&lnode));

    let holder = SelfLoopHolder::install(&lnode);
    let holder_property = lnode
        .lock()
        
        .get_property(InternalProperties::SELF_LOOP_HOLDER)
        .expect("self loop holder property");
    assert!(Arc::ptr_eq(&holder, &holder_property));

    let node_port_count = lnode.lock().ports().len();
    assert_eq!(
        node_port_count,
        holder.lock().sl_port_map().len()
    );
    assert_eq!(
        node_port_count / 2,
        holder.lock().sl_hyper_loops().len()
    );
}

#[test]
fn test_different_self_hyper_loops() {
    let graph = basic_graph_without_self_loops();
    let lnode = graph.lock().layerless_nodes()[1].clone();

    ports(&lnode, 2, 2, 2, 1);

    let port_list = lnode.lock().ports().clone();
    let mut first_loops = BTreeSet::new();
    for i in 1..(port_list.len() / 2) {
        let ledge = edge(&port_list[i - 1], &port_list[i]);
        first_loops.insert(edge_key(&ledge));
    }

    let mut second_loops = BTreeSet::new();
    for i in ((port_list.len() / 2) + 1)..port_list.len() {
        let ledge = edge(&port_list[i - 1], &port_list[i]);
        second_loops.insert(edge_key(&ledge));
    }

    let holder = SelfLoopHolder::install(&lnode);
    let hyper_loops = holder.lock().sl_hyper_loops().clone();
    assert_eq!(2, hyper_loops.len());

    let first_hyper_loop_edges: BTreeSet<usize> = hyper_loops[0]
        .lock()
        
        .sl_edges()
        .iter()
        .map(|sl_edge| {
            let edge = sl_edge
                .lock()
                
                .l_edge()
                .clone();
            edge_key(&edge)
        })
        .collect();
    let second_hyper_loop_edges: BTreeSet<usize> = hyper_loops[1]
        .lock()
        
        .sl_edges()
        .iter()
        .map(|sl_edge| {
            let edge = sl_edge
                .lock()
                
                .l_edge()
                .clone();
            edge_key(&edge)
        })
        .collect();

    let matches_in_order =
        first_hyper_loop_edges == first_loops && second_hyper_loop_edges == second_loops;
    let matches_swapped =
        first_hyper_loop_edges == second_loops && second_hyper_loop_edges == first_loops;

    assert!(matches_in_order || matches_swapped);
}
