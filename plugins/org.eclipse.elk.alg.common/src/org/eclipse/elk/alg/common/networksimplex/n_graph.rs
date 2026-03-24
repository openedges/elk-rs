use super::n_edge::NEdge;
use super::n_node::{NNode, NNodeRef};
use std::collections::VecDeque;

#[derive(Default)]
pub struct NGraph {
    pub nodes: Vec<NNodeRef>,
}

impl NGraph {
    pub fn new() -> Self {
        NGraph { nodes: Vec::new() }
    }

    pub fn write_debug_graph(&self, _file_path: &str) {
        // No-op in Rust port for now.
    }

    pub fn make_connected(&mut self) -> Option<NNodeRef> {
        for (index, node) in self.nodes.iter().enumerate() {
            let mut node_guard = node.lock();
            node_guard.internal_id = index;
        }
        let cc_rep = self.find_con_comp_representatives();
        if cc_rep.len() > 1 {
            return Some(self.create_artificial_root_and_connect(&cc_rep));
        }
        None
    }

    fn create_artificial_root_and_connect(&mut self, nodes_to_connect: &[NNodeRef]) -> NNodeRef {
        let root = NNode::of().create(self);
        for node in nodes_to_connect {
            NEdge::of()
                .delta(0)
                .weight(0.0)
                .source(root.clone())
                .target(node.clone())
                .create();
        }
        root
    }

    fn find_con_comp_representatives(&self) -> Vec<NNodeRef> {
        let mut reps = Vec::new();
        let mut mark = vec![false; self.nodes.len()];
        for node in &self.nodes {
            let idx = node_index(node);
            if idx < mark.len() && !mark[idx] {
                reps.push(node.clone());
                Self::dfs(node, &mut mark);
            }
        }
        reps
    }

    fn dfs(node: &NNodeRef, mark: &mut [bool]) {
        let idx = node_index(node);
        if idx >= mark.len() || mark[idx] {
            return;
        }
        mark[idx] = true;

        // Collect neighbor nodes without allocating connected_edges() Vec.
        // Iterate incoming then outgoing edges directly (Java parity order).
        let neighbors: Vec<NNodeRef> = {
            let node_guard = node.lock();
            let mut nbrs =
                Vec::with_capacity(node_guard.incoming_edges().len() + node_guard.outgoing_edges().len());
            for edge in node_guard.incoming_edges() {
                let eg = edge.lock();
                nbrs.push(eg.other(node));
            }
            for edge in node_guard.outgoing_edges() {
                let eg = edge.lock();
                nbrs.push(eg.other(node));
            }
            nbrs
        };
        for other in neighbors {
            Self::dfs(&other, mark);
        }
    }

    pub fn is_acyclic(&self) -> bool {
        for (index, node) in self.nodes.iter().enumerate() {
            let mut node_guard = node.lock();
            node_guard.internal_id = index;
        }

        let mut incident = vec![0usize; self.nodes.len()];
        let mut layer = vec![0usize; self.nodes.len()];
        for node in &self.nodes {
            let idx = node_index(node);
            let node_guard = node.lock();
            incident[idx] += node_guard.incoming_edges().len();
        }

        let mut roots = VecDeque::new();
        for node in &self.nodes {
            let node_guard = node.lock();
            if node_guard.incoming_edges().is_empty() {
                roots.push_back(node.clone());
            }
        }

        if roots.is_empty() && !self.nodes.is_empty() {
            return false;
        }

        while let Some(node) = roots.pop_front() {
            let node_idx = node_index(&node);
            // Collect (target, target_idx) pairs under single node lock, then process
            let targets: Vec<(NNodeRef, usize)> = {
                let node_guard = node.lock();
                node_guard
                    .outgoing_edges()
                    .iter()
                    .map(|edge| {
                        let eg = edge.lock();
                        let tgt = eg.target.clone();
                        let tgt_idx = tgt.lock().internal_id;
                        (tgt, tgt_idx)
                    })
                    .collect()
            };
            for (target, target_idx) in targets {
                layer[target_idx] = layer[target_idx].max(layer[node_idx] + 1);
                if incident[target_idx] > 0 {
                    incident[target_idx] -= 1;
                }
                if incident[target_idx] == 0 {
                    roots.push_back(target);
                }
            }
        }

        for node in &self.nodes {
            // Collect (src_idx, tgt_idx) under node lock. For outgoing edges,
            // source == node (already locked) so use node_guard.internal_id directly.
            // target is a different node (no self-loops) so tgt.lock() is safe.
            let edges_info: Vec<(usize, usize)> = {
                let node_guard = node.lock();
                let src_idx = node_guard.internal_id;
                node_guard
                    .outgoing_edges()
                    .iter()
                    .map(|edge| {
                        let eg = edge.lock();
                        let tgt_idx = eg.target.lock().internal_id;
                        (src_idx, tgt_idx)
                    })
                    .collect()
            };
            for (source_idx, target_idx) in edges_info {
                if layer[target_idx] <= layer[source_idx] {
                    return false;
                }
            }
        }

        true
    }
}

fn node_index(node: &NNodeRef) -> usize {
    node.lock().internal_id
}
