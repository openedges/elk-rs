use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use crate::org::eclipse::elk::alg::layered::graph::LPortRef;
use crate::org::eclipse::elk::alg::layered::intermediate::loops::SelfLoopEdgeRef;

pub type SelfLoopPortRef = Arc<Mutex<SelfLoopPort>>;

pub struct SelfLoopPort {
    l_port: LPortRef,
    had_only_self_loops: bool,
    incoming_sl_edges: Vec<SelfLoopEdgeRef>,
    outgoing_sl_edges: Vec<SelfLoopEdgeRef>,
    hidden: bool,
}

impl SelfLoopPort {
    pub fn new(l_port: &LPortRef) -> SelfLoopPortRef {
        let connected = l_port
            .lock()
            .ok()
            .map(|port_guard| port_guard.connected_edges())
            .unwrap_or_default();
        let had_only_self_loops = !connected.is_empty()
            && connected.iter().all(|edge| {
                edge.lock()
                    .ok()
                    .map(|edge_guard| edge_guard.is_self_loop())
                    .unwrap_or(false)
            });

        Arc::new(Mutex::new(SelfLoopPort {
            l_port: l_port.clone(),
            had_only_self_loops,
            incoming_sl_edges: Vec::new(),
            outgoing_sl_edges: Vec::new(),
            hidden: false,
        }))
    }

    pub fn l_port(&self) -> &LPortRef {
        &self.l_port
    }

    pub fn had_only_self_loops(&self) -> bool {
        self.had_only_self_loops
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    pub fn incoming_sl_edges(&self) -> &Vec<SelfLoopEdgeRef> {
        &self.incoming_sl_edges
    }

    pub fn incoming_sl_edges_mut(&mut self) -> &mut Vec<SelfLoopEdgeRef> {
        &mut self.incoming_sl_edges
    }

    pub fn outgoing_sl_edges(&self) -> &Vec<SelfLoopEdgeRef> {
        &self.outgoing_sl_edges
    }

    pub fn outgoing_sl_edges_mut(&mut self) -> &mut Vec<SelfLoopEdgeRef> {
        &mut self.outgoing_sl_edges
    }

    pub fn sl_net_flow(&self) -> isize {
        self.incoming_sl_edges.len() as isize - self.outgoing_sl_edges.len() as isize
    }
}
