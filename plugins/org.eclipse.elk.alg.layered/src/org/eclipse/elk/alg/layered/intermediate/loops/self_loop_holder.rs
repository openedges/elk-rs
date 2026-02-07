use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::EnumSetType;

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::loops::{
    SelfHyperLoop, SelfHyperLoopRef, SelfLoopEdge, SelfLoopPort, SelfLoopPortRef,
};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;

pub type SelfLoopHolderRef = Arc<Mutex<SelfLoopHolder>>;

type SelfLoopPortMap = Vec<(LPortRef, SelfLoopPortRef)>;

pub struct SelfLoopHolder {
    l_node: LNodeRef,
    sl_hyper_loops: Vec<SelfHyperLoopRef>,
    sl_ports: SelfLoopPortMap,
    are_ports_hidden: bool,
    routing_slot_count: Vec<i32>,
}

impl SelfLoopHolder {
    fn new(node: &LNodeRef) -> SelfLoopHolder {
        SelfLoopHolder {
            l_node: node.clone(),
            sl_hyper_loops: Vec::new(),
            sl_ports: Vec::new(),
            are_ports_hidden: false,
            routing_slot_count: vec![0; PortSide::variants().len()],
        }
    }

    pub fn install(node: &LNodeRef) -> SelfLoopHolderRef {
        assert!(Self::needs_self_loop_processing(node));

        let holder = Arc::new(Mutex::new(SelfLoopHolder::new(node)));

        if let Ok(mut node_guard) = node.lock() {
            node_guard.set_property(InternalProperties::SELF_LOOP_HOLDER, Some(holder.clone()));
        }

        Self::initialize(&holder);
        holder
    }

    pub fn needs_self_loop_processing(node: &LNodeRef) -> bool {
        let (is_normal, outgoing_edges) = node
            .lock()
            .ok()
            .map(|node_guard| {
                (
                    node_guard.node_type() == NodeType::Normal,
                    node_guard.outgoing_edges().clone(),
                )
            })
            .unwrap_or((false, Vec::new()));

        is_normal
            && outgoing_edges.iter().any(|edge| {
                edge.lock()
                    .ok()
                    .map(|edge_guard| edge_guard.is_self_loop())
                    .unwrap_or(false)
            })
    }

    fn initialize(holder: &SelfLoopHolderRef) {
        let node = holder
            .lock()
            .ok()
            .map(|holder_guard| holder_guard.l_node.clone())
            .unwrap_or_else(|| panic!("self loop holder lock poisoned"));

        let outgoing_edges = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.outgoing_edges().clone())
            .unwrap_or_default();

        for edge in outgoing_edges {
            let is_self_loop = edge
                .lock()
                .ok()
                .map(|edge_guard| edge_guard.is_self_loop())
                .unwrap_or(false);
            if !is_self_loop {
                continue;
            }

            let (source_port, target_port) = edge
                .lock()
                .ok()
                .map(|edge_guard| (edge_guard.source(), edge_guard.target()))
                .unwrap_or((None, None));
            let (Some(source_port), Some(target_port)) = (source_port, target_port) else {
                continue;
            };

            let sl_source = Self::self_loop_port_for(holder, &source_port);
            let sl_target = Self::self_loop_port_for(holder, &target_port);
            let _ = SelfLoopEdge::new(&edge, &sl_source, &sl_target);
        }

        let sl_ports = holder
            .lock()
            .ok()
            .map(|holder_guard| {
                holder_guard
                    .sl_ports
                    .iter()
                    .map(|(_, sl_port)| sl_port.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut visited: HashSet<usize> = HashSet::new();
        for sl_port in sl_ports {
            let key = Arc::as_ptr(&sl_port) as usize;
            if visited.contains(&key) {
                continue;
            }

            let hyper_loop = Self::initialize_hyper_loop(&sl_port, &mut visited);
            if let Ok(mut holder_guard) = holder.lock() {
                holder_guard.sl_hyper_loops.push(hyper_loop);
            }
        }
    }

    fn self_loop_port_for(holder: &SelfLoopHolderRef, l_port: &LPortRef) -> SelfLoopPortRef {
        if let Ok(holder_guard) = holder.lock() {
            if let Some((_, sl_port)) = holder_guard
                .sl_ports
                .iter()
                .find(|(existing_port, _)| Arc::ptr_eq(existing_port, l_port))
            {
                return sl_port.clone();
            }
        }

        let sl_port = SelfLoopPort::new(l_port);
        if let Ok(mut holder_guard) = holder.lock() {
            holder_guard.sl_ports.push((l_port.clone(), sl_port.clone()));
        }
        sl_port
    }

    fn initialize_hyper_loop(
        start_port: &SelfLoopPortRef,
        visited: &mut HashSet<usize>,
    ) -> SelfHyperLoopRef {
        let sl_loop = SelfHyperLoop::new();
        let mut queue = VecDeque::new();

        queue.push_back(start_port.clone());
        visited.insert(Arc::as_ptr(start_port) as usize);

        while let Some(current_sl_port) = queue.pop_front() {
            let (outgoing, incoming) = current_sl_port
                .lock()
                .ok()
                .map(|port_guard| {
                    (
                        port_guard.outgoing_sl_edges().clone(),
                        port_guard.incoming_sl_edges().clone(),
                    )
                })
                .unwrap_or_default();

            for sl_edge in outgoing.into_iter().chain(incoming) {
                SelfHyperLoop::add_self_loop_edge(&sl_loop, &sl_edge);

                let (source_port, target_port) = sl_edge
                    .lock()
                    .ok()
                    .map(|edge_guard| (edge_guard.sl_source().clone(), edge_guard.sl_target().clone()))
                    .unwrap_or_else(|| panic!("self loop edge lock poisoned"));

                let source_key = Arc::as_ptr(&source_port) as usize;
                if visited.insert(source_key) {
                    queue.push_back(source_port);
                }

                let target_key = Arc::as_ptr(&target_port) as usize;
                if visited.insert(target_key) {
                    queue.push_back(target_port);
                }
            }
        }

        sl_loop
    }

    pub fn l_node(&self) -> &LNodeRef {
        &self.l_node
    }

    pub fn sl_hyper_loops(&self) -> &Vec<SelfHyperLoopRef> {
        &self.sl_hyper_loops
    }

    pub fn sl_port_map(&self) -> &SelfLoopPortMap {
        &self.sl_ports
    }

    pub fn sl_port_values(&self) -> Vec<SelfLoopPortRef> {
        self.sl_ports
            .iter()
            .map(|(_, sl_port)| sl_port.clone())
            .collect()
    }

    pub fn are_ports_hidden(&self) -> bool {
        self.are_ports_hidden
    }

    pub fn set_ports_hidden(&mut self, hidden: bool) {
        self.are_ports_hidden = hidden;
    }

    pub fn routing_slot_count(&self) -> &[i32] {
        &self.routing_slot_count
    }

    pub fn routing_slot_count_mut(&mut self) -> &mut Vec<i32> {
        &mut self.routing_slot_count
    }

    pub fn all_self_loop_edges(&self) -> Vec<LEdgeRef> {
        self.sl_hyper_loops
            .iter()
            .flat_map(|sl_loop| {
                sl_loop
                    .lock()
                    .ok()
                    .map(|loop_guard| {
                        loop_guard
                            .sl_edges()
                            .iter()
                            .filter_map(|sl_edge| {
                                sl_edge
                                    .lock()
                                    .ok()
                                    .map(|edge_guard| edge_guard.l_edge().clone())
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }
}
