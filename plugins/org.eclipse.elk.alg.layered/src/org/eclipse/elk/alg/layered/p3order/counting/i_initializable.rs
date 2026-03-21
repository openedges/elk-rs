use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef};

pub trait IInitializable {
    fn init_at_layer_level(&mut self, _layer_index: usize, _node_order: &[Vec<LNodeRef>]) {}

    fn init_at_node_level(
        &mut self,
        _layer_index: usize,
        _node_index: usize,
        _node_order: &[Vec<LNodeRef>],
    ) {
    }

    fn init_at_port_level(
        &mut self,
        _layer_index: usize,
        _node_index: usize,
        _port_index: usize,
        _node_order: &[Vec<LNodeRef>],
    ) {
    }

    fn init_at_edge_level(
        &mut self,
        _layer_index: usize,
        _node_index: usize,
        _port_index: usize,
        _edge_index: usize,
        _edge: &LEdgeRef,
        _node_order: &[Vec<LNodeRef>],
    ) {
    }

    fn init_after_traversal(&mut self) {}
}

pub fn init(initializables: &mut [&mut dyn IInitializable], order: &[Vec<LNodeRef>]) {
    let trace = ElkTrace::global().crossmin;
    if trace {
        eprintln!("crossmin:init_initializables layers={}", order.len());
    }
    for (layer_index, layer_nodes) in order.iter().enumerate() {
        if trace {
            eprintln!("crossmin:init_initializables layer {}", layer_index);
        }
        for initable in initializables.iter_mut() {
            initable.init_at_layer_level(layer_index, order);
        }
        for (node_index, node) in layer_nodes.iter().enumerate() {
            for initable in initializables.iter_mut() {
                initable.init_at_node_level(layer_index, node_index, order);
            }
            let ports = node
                .lock_ok()
                .map(|node_guard| node_guard.ports().clone())
                .unwrap_or_default();
            for (port_index, port) in ports.iter().enumerate() {
                for initable in initializables.iter_mut() {
                    initable.init_at_port_level(layer_index, node_index, port_index, order);
                }
                let edges = port
                    .lock_ok()
                    .map(|port_guard| port_guard.connected_edges().clone())
                    .unwrap_or_default();
                for (edge_index, edge) in edges.iter().enumerate() {
                    for initable in initializables.iter_mut() {
                        initable.init_at_edge_level(
                            layer_index,
                            node_index,
                            port_index,
                            edge_index,
                            edge,
                            order,
                        );
                    }
                }
            }
        }
    }

    for initable in initializables.iter_mut() {
        initable.init_after_traversal();
    }
    if trace {
        eprintln!("crossmin:init_initializables done");
    }
}
