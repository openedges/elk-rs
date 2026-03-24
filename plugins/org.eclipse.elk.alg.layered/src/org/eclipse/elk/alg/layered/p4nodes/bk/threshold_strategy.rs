use std::collections::{HashSet, VecDeque};

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::LEdgeRef;

use super::aligned_layout::{BKAlignedLayout, HDirection, VDirection};
use super::neighborhood_information::NeighborhoodInformation;
use super::util::{node_id, port_node_id_a, port_offset_y_a};

const THRESHOLD: f64 = f64::MAX;
const EPSILON: f64 = 0.0001;

pub trait ThresholdStrategy {
    fn init(&mut self);
    fn finish_block(&mut self, root: usize);
    fn calculate_threshold(
        &mut self,
        bal: &mut BKAlignedLayout,
        old_thresh: f64,
        block_root: usize,
        current_node: usize,
    ) -> f64;
    fn post_process(&mut self, bal: &mut BKAlignedLayout, ni: &NeighborhoodInformation);
}

pub struct NullThresholdStrategy;

impl NullThresholdStrategy {
    pub fn new() -> Self {
        NullThresholdStrategy
    }
}

impl Default for NullThresholdStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ThresholdStrategy for NullThresholdStrategy {
    fn init(&mut self) {}

    fn finish_block(&mut self, _root: usize) {}

    fn calculate_threshold(
        &mut self,
        bal: &mut BKAlignedLayout,
        _old_thresh: f64,
        _block_root: usize,
        _current_node: usize,
    ) -> f64 {
        match bal.vdir {
            Some(VDirection::Up) => f64::INFINITY,
            _ => f64::NEG_INFINITY,
        }
    }

    fn post_process(&mut self, _bal: &mut BKAlignedLayout, _ni: &NeighborhoodInformation) {}
}

pub struct SimpleThresholdStrategy {
    block_finished: HashSet<usize>,
    post_process_queue: VecDeque<Postprocessable>,
    post_process_stack: Vec<Postprocessable>,
}

impl SimpleThresholdStrategy {
    pub fn new() -> Self {
        SimpleThresholdStrategy {
            block_finished: HashSet::new(),
            post_process_queue: VecDeque::new(),
            post_process_stack: Vec::new(),
        }
    }

    fn pick_edge(&mut self, bal: &BKAlignedLayout, mut pp: Postprocessable) -> Postprocessable {
        let free_node = &bal.nodes_by_id[pp.free];
        let edges = if pp.is_root {
            if bal.hdir == Some(HDirection::Right) {
                free_node
                    .lock().incoming_edges()
            } else {
                free_node
                    .lock().outgoing_edges()
            }
        } else if bal.hdir == Some(HDirection::Left) {
            free_node
                .lock().incoming_edges()
        } else {
            free_node
                .lock().outgoing_edges()
        };

        let mut has_edges = false;
        for edge in edges {
            let only_dummies = bal.od[bal.root[pp.free]];
            if !only_dummies {
                let in_layer = edge
                    .lock().is_in_layer_edge();
                if in_layer {
                    continue;
                }
            }

            if bal.su[bal.root[pp.free]] {
                continue;
            }

            has_edges = true;

            let other_node_id = {
                let other = edge.lock().other_node(free_node);
                Some(node_id(&other))
            };
            if let Some(other_node_id) = other_node_id {
                if other_node_id >= bal.root.len() {
                    continue;
                }
                let other_root = bal.root[other_node_id];
                if self.block_finished.contains(&other_root) {
                    pp.has_edges = true;
                    pp.edge = Some(edge);
                    return pp;
                }
            }
        }

        pp.has_edges = has_edges;
        pp.edge = None;
        pp
    }

    fn get_bound(&mut self, bal: &mut BKAlignedLayout, block_node: usize, is_root: bool) -> f64 {
        let trace = ElkTrace::global().bk_thresh;
        let invalid = match bal.vdir {
            Some(VDirection::Up) => f64::INFINITY,
            _ => f64::NEG_INFINITY,
        };

        let pick = self.pick_edge(bal, Postprocessable::new(block_node, is_root));

        if pick.edge.is_none() && pick.has_edges {
            if trace {
                eprintln!(
                    "bk-thresh: queue free={} is_root={} has_edges=true reason=no_finished_neighbor",
                    block_node, is_root
                );
            }
            self.post_process_queue.push_back(pick);
            return invalid;
        }

        let Some(edge) = pick.edge else {
            return invalid;
        };

        let (left_port, right_port) = {
            let edge_guard = edge.lock();
            (edge_guard.source().unwrap(), edge_guard.target().unwrap())
        };

        let threshold = if is_root {
            let root_port = if bal.hdir == Some(HDirection::Right) {
                right_port.clone()
            } else {
                left_port.clone()
            };
            let other_port = if bal.hdir == Some(HDirection::Right) {
                left_port.clone()
            } else {
                right_port.clone()
            };
            let other_node_id = port_node_id_a(&bal.sync, &other_port);
            if other_node_id >= bal.root.len() {
                return invalid;
            }
            let root_node_id = port_node_id_a(&bal.sync, &root_port);
            if root_node_id >= bal.root.len() {
                return invalid;
            }
            let other_root = bal.root[other_node_id];
            bal.y[other_root].unwrap_or(0.0)
                + bal.inner_shift[other_node_id]
                + port_offset_y_a(&bal.sync, &other_port)
                - bal.inner_shift[root_node_id]
                - port_offset_y_a(&bal.sync, &root_port)
        } else {
            let root_port = if bal.hdir == Some(HDirection::Left) {
                right_port.clone()
            } else {
                left_port.clone()
            };
            let other_port = if bal.hdir == Some(HDirection::Left) {
                left_port.clone()
            } else {
                right_port.clone()
            };
            let other_node_id = port_node_id_a(&bal.sync, &other_port);
            if other_node_id >= bal.root.len() {
                return invalid;
            }
            let root_node_id = port_node_id_a(&bal.sync, &root_port);
            if root_node_id >= bal.root.len() {
                return invalid;
            }
            let other_root = bal.root[other_node_id];
            bal.y[other_root].unwrap_or(0.0)
                + bal.inner_shift[other_node_id]
                + port_offset_y_a(&bal.sync, &other_port)
                - bal.inner_shift[root_node_id]
                - port_offset_y_a(&bal.sync, &root_port)
        };

        let left_node_id = port_node_id_a(&bal.sync, &left_port);
        if left_node_id >= bal.root.len() {
            return invalid;
        }
        let right_node_id = port_node_id_a(&bal.sync, &right_port);
        if right_node_id >= bal.root.len() {
            return invalid;
        }
        let left_root = bal.root[left_node_id];
        let right_root = bal.root[right_node_id];
        bal.su[left_root] = true;
        bal.su[right_root] = true;
        if trace {
            eprintln!(
                "bk-thresh: bound free={} is_root={} threshold={threshold:.3} left_root={} right_root={}",
                block_node, is_root, left_root, right_root
            );
        }

        threshold
    }

    fn process(
        &self,
        bal: &mut BKAlignedLayout,
        ni: &NeighborhoodInformation,
        pp: &Postprocessable,
    ) -> bool {
        let trace = ElkTrace::global().bk_thresh;
        let edge = pp.edge.as_ref().expect("processable edge missing");
        let (source_port, target_port) = {
            let edge_guard = edge.lock();
            (edge_guard.source().unwrap(), edge_guard.target().unwrap())
        };

        let free_id = pp.free;
        let source_node_id = port_node_id_a(&bal.sync, &source_port);
        let (fix, block) = if source_node_id == free_id {
            (target_port, source_port)
        } else {
            (source_port, target_port)
        };

        let block_node_id = port_node_id_a(&bal.sync, &block);
        let delta = bal.calculate_delta(&fix, &block);
        if trace {
            eprintln!(
                "bk-thresh: process free={} is_root={} block_node={} delta={delta:.3}",
                free_id, pp.is_root, block_node_id
            );
        }

        if delta > 0.0 && delta < THRESHOLD {
            let available_space = bal.check_space_above(block_node_id, delta, ni);
            debug_assert!(available_space.abs() < EPSILON || available_space >= 0.0);
            bal.shift_block(block_node_id, -available_space);
            if trace {
                eprintln!(
                    "bk-thresh: process-up block_node={} delta={delta:.3} available={available_space:.3}",
                    block_node_id
                );
            }
            return available_space > 0.0;
        } else if delta < 0.0 && -delta < THRESHOLD {
            let available_space = bal.check_space_below(block_node_id, -delta, ni);
            debug_assert!(available_space.abs() < EPSILON || available_space >= 0.0);
            bal.shift_block(block_node_id, available_space);
            if trace {
                eprintln!(
                    "bk-thresh: process-down block_node={} delta={delta:.3} available={available_space:.3}",
                    block_node_id
                );
            }
            return available_space > 0.0;
        }

        false
    }
}

impl Default for SimpleThresholdStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl ThresholdStrategy for SimpleThresholdStrategy {
    fn init(&mut self) {
        self.block_finished.clear();
        self.post_process_queue.clear();
        self.post_process_stack.clear();
    }

    fn finish_block(&mut self, root: usize) {
        self.block_finished.insert(root);
    }

    fn calculate_threshold(
        &mut self,
        bal: &mut BKAlignedLayout,
        old_thresh: f64,
        block_root: usize,
        current_node: usize,
    ) -> f64 {
        let is_root = block_root == current_node;
        let is_last = bal.align[current_node] == block_root;

        if !(is_root || is_last) {
            return old_thresh;
        }

        let mut t = old_thresh;
        if is_root {
            t = self.get_bound(bal, block_root, true);
        }
        if t.is_infinite() && is_last {
            t = self.get_bound(bal, current_node, false);
        }

        t
    }

    fn post_process(&mut self, bal: &mut BKAlignedLayout, ni: &NeighborhoodInformation) {
        let trace = ElkTrace::global().bk_thresh;
        while let Some(pp) = self.post_process_queue.pop_front() {
            let pick = self.pick_edge(bal, pp);
            let Some(edge) = pick.edge.clone() else {
                if trace {
                    eprintln!(
                        "bk-thresh: dequeue free={} is_root={} no-edge",
                        pick.free, pick.is_root
                    );
                }
                continue;
            };

            let only_dummies = bal.od[bal.root[pick.free]];
            let in_layer = edge
                .lock().is_in_layer_edge();
            if !only_dummies && in_layer {
                continue;
            }

            let moved = self.process(bal, ni, &pick);
            if trace {
                eprintln!(
                    "bk-thresh: dequeue free={} is_root={} moved={moved}",
                    pick.free, pick.is_root
                );
            }
            if !moved {
                self.post_process_stack.push(pick);
            }
        }

        while let Some(pp) = self.post_process_stack.pop() {
            if trace {
                eprintln!(
                    "bk-thresh: stack-pop free={} is_root={}",
                    pp.free, pp.is_root
                );
            }
            let _ = self.process(bal, ni, &pp);
        }
    }
}

#[derive(Clone)]
struct Postprocessable {
    free: usize,
    is_root: bool,
    has_edges: bool,
    edge: Option<LEdgeRef>,
}

impl Postprocessable {
    fn new(free: usize, is_root: bool) -> Self {
        Postprocessable {
            free,
            is_root,
            has_edges: false,
            edge: None,
        }
    }
}
