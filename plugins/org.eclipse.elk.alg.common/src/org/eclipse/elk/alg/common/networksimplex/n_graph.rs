use std::collections::VecDeque;
use super::n_edge::NEdge;
use super::n_node::{NNode, NNodeRef};

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
            if let Ok(mut node_guard) = node.lock() {
                node_guard.internal_id = index;
            }
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

        let edges = match node.lock() {
            Ok(node_guard) => node_guard.connected_edges(),
            Err(_) => Vec::new(),
        };
        for edge in edges {
            let other = edge.lock().ok().map(|edge_guard| edge_guard.other(node));
            if let Some(other) = other {
                Self::dfs(&other, mark);
            }
        }
    }

    pub fn is_acyclic(&self) -> bool {
        for (index, node) in self.nodes.iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.internal_id = index;
            }
        }

        let mut incident = vec![0usize; self.nodes.len()];
        let mut layer = vec![0usize; self.nodes.len()];
        for node in &self.nodes {
            let idx = node_index(node);
            if let Ok(node_guard) = node.lock() {
                incident[idx] += node_guard.incoming_edges().len();
            }
        }

        let mut roots = VecDeque::new();
        for node in &self.nodes {
            if let Ok(node_guard) = node.lock() {
                if node_guard.incoming_edges().is_empty() {
                    roots.push_back(node.clone());
                }
            }
        }

        if roots.is_empty() && !self.nodes.is_empty() {
            return false;
        }

        while let Some(node) = roots.pop_front() {
            let node_idx = node_index(&node);
            let outgoing = match node.lock() {
                Ok(node_guard) => node_guard.outgoing_edges().clone(),
                Err(_) => Vec::new(),
            };
            for edge in outgoing {
                let target = edge.lock().ok().map(|edge_guard| edge_guard.target.clone());
                let Some(target) = target else { continue; };
                let target_idx = node_index(&target);
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
            let outgoing = match node.lock() {
                Ok(node_guard) => node_guard.outgoing_edges().clone(),
                Err(_) => Vec::new(),
            };
            for edge in outgoing {
                let (source, target) = match edge.lock() {
                    Ok(edge_guard) => (edge_guard.source.clone(), edge_guard.target.clone()),
                    Err(_) => continue,
                };
                let source_idx = node_index(&source);
                let target_idx = node_index(&target);
                if layer[target_idx] <= layer[source_idx] {
                    return false;
                }
            }
        }

        true
    }
}

fn node_index(node: &NNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.internal_id)
        .unwrap_or(0)
}
