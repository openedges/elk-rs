use std::collections::VecDeque;
use std::sync::Arc;

use rustc_hash::FxHashSet;

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;
use org_eclipse_elk_core::org::eclipse::elk::core::util::progress_monitor::BasicProgressMonitor;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use super::n_edge::NEdgeRef;
use super::n_graph::NGraph;
use super::n_node::NNodeRef;

const REMOVE_SUBTREES_THRESH: usize = 40;
const FUZZY_ST_ZERO: f64 = -1e-10;

pub struct NetworkSimplex<'a> {
    graph: &'a mut NGraph,
    previous_layering_node_counts: Option<Vec<i32>>,
    balance: bool,
    iteration_limit: i32,
    edges: Vec<NEdgeRef>,
    tree_edge_ids: Vec<usize>,
    sources: Vec<NNodeRef>,
    edge_visited: Vec<bool>,
    post_order: i32,
    po_id: Vec<i32>,
    lowest_po_id: Vec<i32>,
    cutvalue: Vec<f64>,
    // Integral cut sums admit exact pivot updates without floating-point reassociation.
    integral_cutvalues: bool,
    subtree_nodes_stack: Option<VecDeque<Pair<NNodeRef, NEdgeRef>>>,
    // Cached edge properties (indexed by edge internal_id, constant after initialize)
    e_src: Vec<NNodeRef>,
    e_tgt: Vec<NNodeRef>,
    e_src_id: Vec<usize>,
    e_tgt_id: Vec<usize>,
    e_delta: Vec<i32>,
    e_weight: Vec<f64>,
    e_tree: Vec<bool>,
    // Per-node adjacency: connected edge internal_ids (incoming first, then outgoing)
    node_edges: Vec<Vec<usize>>,
    // Tree-only adjacency in the same incoming/outgoing edge order.
    tree_edges: Vec<Vec<usize>>,
    // Cached node properties (indexed by node internal_id, synced during algorithm)
    n_layer: Vec<i32>,
    n_tree_node: Vec<bool>,
    // Reusable stack for iterative postorder traversal (nid, adj_cursor, lowest)
    postorder_stack: Vec<(usize, usize, i32)>,
}

impl<'a> NetworkSimplex<'a> {
    pub fn for_graph(graph: &'a mut NGraph) -> Self {
        NetworkSimplex {
            graph,
            previous_layering_node_counts: None,
            balance: false,
            iteration_limit: i32::MAX,
            edges: Vec::new(),
            tree_edge_ids: Vec::new(),
            sources: Vec::new(),
            edge_visited: Vec::new(),
            post_order: 1,
            po_id: Vec::new(),
            lowest_po_id: Vec::new(),
            cutvalue: Vec::new(),
            integral_cutvalues: false,
            subtree_nodes_stack: None,
            e_src: Vec::new(),
            e_tgt: Vec::new(),
            e_src_id: Vec::new(),
            e_tgt_id: Vec::new(),
            e_delta: Vec::new(),
            e_weight: Vec::new(),
            e_tree: Vec::new(),
            node_edges: Vec::new(),
            tree_edges: Vec::new(),
            n_layer: Vec::new(),
            n_tree_node: Vec::new(),
            postorder_stack: Vec::new(),
        }
    }

    pub fn with_balancing(&mut self, do_balance: bool) -> &mut Self {
        self.balance = do_balance;
        self
    }

    pub fn with_previous_layering(&mut self, counts: Option<&[i32]>) -> &mut Self {
        self.previous_layering_node_counts = counts.map(|slice| slice.to_vec());
        self
    }

    pub fn with_iteration_limit(&mut self, limit: i32) -> &mut Self {
        self.iteration_limit = limit;
        self
    }

    pub fn execute(&mut self) {
        let mut monitor = BasicProgressMonitor::new();
        self.execute_with_monitor(&mut monitor);
    }

    pub fn execute_with_monitor(&mut self, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Network simplex", 1.0);

        if self.graph.nodes.is_empty() {
            monitor.done();
            return;
        }

        for node in &self.graph.nodes {
            let mut node_guard = node.lock();
            node_guard.layer = 0;
        }

        let remove_subtrees = self.graph.nodes.len() >= REMOVE_SUBTREES_THRESH;
        if remove_subtrees {
            self.remove_subtrees();
        }

        self.initialize();
        self.feasible_tree();

        let mut leave_opt = self.leave_edge();
        let mut iter = 0;
        while let Some(leave_id) = leave_opt {
            if iter >= self.iteration_limit {
                break;
            }
            if let Some(enter_id) = self.enter_edge(leave_id) {
                self.exchange(leave_id, enter_id);
            } else {
                if ElkTrace::global().network_simplex {
                    eprintln!(
                        "[network-simplex] break: missing entering edge at iter={iter} leave_edge_id={}",
                        leave_id
                    );
                }
                break;
            }
            leave_opt = self.leave_edge();
            iter += 1;
        }

        // Pivots only need the dense layer array. Publish it once before the
        // object-based subtree and balancing passes read node layers.
        for (nid, node) in self.graph.nodes.iter().enumerate() {
            node.lock().layer = self.n_layer[nid];
        }
        if remove_subtrees {
            self.reattach_subtrees();
        }

        if self.balance {
            let filling = self.normalize();
            self.balance_layers(&filling);
        } else {
            self.normalize();
        }

        self.dispose();
        monitor.done();
    }

    fn initialize(&mut self) {
        let num_nodes = self.graph.nodes.len();
        // tree_node sync to underlying NNode deferred — algorithm uses SoA cache only
        self.po_id = vec![0; num_nodes];
        self.lowest_po_id = vec![0; num_nodes];
        self.sources.clear();

        let mut edges = Vec::new();
        for (index, node) in self.graph.nodes.iter().enumerate() {
            let mut node_guard = node.lock();
            node_guard.internal_id = index;
            if node_guard.incoming_edges().is_empty() {
                self.sources.push(node.clone());
            }
            edges.extend(node_guard.outgoing_edges().clone());
        }

        // Build edge property cache (one lock per edge, all properties extracted)
        let num_edges = edges.len();
        self.e_src.clear();
        self.e_tgt.clear();
        self.e_src_id.clear();
        self.e_tgt_id.clear();
        self.e_delta.clear();
        self.e_weight.clear();
        self.e_tree.clear();
        self.e_src.reserve(num_edges);
        self.e_tgt.reserve(num_edges);
        self.e_src_id.resize(num_edges, 0);
        self.e_tgt_id.resize(num_edges, 0);
        self.e_delta.resize(num_edges, 0);
        self.e_weight.resize(num_edges, 0.0);
        self.e_tree.resize(num_edges, false);

        for (index, edge) in edges.iter().enumerate() {
            let mut edge_guard = edge.lock();
            edge_guard.internal_id = index;
            self.e_src.push(edge_guard.source());
            self.e_tgt.push(edge_guard.target());
            self.e_delta[index] = edge_guard.delta;
            self.e_weight[index] = edge_guard.weight;
            self.e_tree[index] = false;
            // source/target internal_ids are already set above
            let src_id = edge_guard.source().lock().internal_id;
            let tgt_id = edge_guard.target().lock().internal_id;
            self.e_src_id[index] = src_id;
            self.e_tgt_id[index] = tgt_id;
        }

        // Every signed cut sum stays exactly representable, including the
        // intermediate sum in a pivot update. Fractional weights retain the
        // original leaf-elimination arithmetic and its rounding/tie behaviour.
        self.integral_cutvalues = self
            .e_weight
            .iter()
            .try_fold(0.0, |sum, weight| {
                let sum = sum + weight.abs();
                (weight.is_finite() && weight.fract() == 0.0 && sum <= (1_u64 << 52) as f64)
                    .then_some(sum)
            })
            .is_some();

        // Build node property cache (layers are all 0 at this point, tree_node all false)
        self.n_layer.clear();
        self.n_layer.resize(num_nodes, 0);
        self.n_tree_node.clear();
        self.n_tree_node.resize(num_nodes, false);

        // Build per-node adjacency list (incoming first, then outgoing — Java parity)
        self.node_edges.clear();
        self.node_edges.resize(num_nodes, Vec::new());
        self.tree_edges = vec![Vec::new(); num_nodes];
        for eid in 0..num_edges {
            self.node_edges[self.e_tgt_id[eid]].push(eid); // incoming to target
        }
        for eid in 0..num_edges {
            self.node_edges[self.e_src_id[eid]].push(eid); // outgoing from source
        }

        if self.cutvalue.len() < num_edges {
            self.cutvalue = vec![0.0; num_edges];
            self.edge_visited = vec![false; num_edges];
        } else {
            self.edge_visited.fill(false);
        }

        self.edges = edges;
        self.tree_edge_ids.clear();
        self.post_order = 1;
    }

    fn dispose(&mut self) {
        self.cutvalue.clear();
        self.edges.clear();
        self.tree_edge_ids.clear();
        self.edge_visited.clear();
        self.e_src.clear();
        self.e_tgt.clear();
        self.e_src_id.clear();
        self.e_tgt_id.clear();
        self.e_delta.clear();
        self.e_weight.clear();
        self.e_tree.clear();
        self.node_edges.clear();
        self.tree_edges.clear();
        self.n_layer.clear();
        self.n_tree_node.clear();
        self.lowest_po_id.clear();
        self.po_id.clear();
        self.sources.clear();
        self.subtree_nodes_stack = None;
    }

    fn feasible_tree(&mut self) {
        self.layering_topological_numbering();
        if self.edges.is_empty() {
            return;
        }

        self.edge_visited.fill(false);
        while !self.graph.nodes.is_empty() {
            let count = self.tight_tree_dfs(0);
            if count >= self.graph.nodes.len() {
                break;
            }
            let Some(eid) = self.minimal_slack() else {
                break;
            };
            let src_layer = self.n_layer[self.e_src_id[eid]];
            let tgt_layer = self.n_layer[self.e_tgt_id[eid]];
            let tgt_tree = self.n_tree_node[self.e_tgt_id[eid]];
            let mut slack = tgt_layer - src_layer - self.e_delta[eid];
            if tgt_tree {
                slack = -slack;
            }
            for (i, layer) in self.n_layer.iter_mut().enumerate() {
                if self.n_tree_node[i] {
                    *layer += slack;
                }
            }
            self.edge_visited.fill(false);
        }

        for (nid, edges) in self.node_edges.iter().enumerate() {
            self.tree_edges[nid].extend(edges.iter().copied().filter(|&eid| self.e_tree[eid]));
        }
        self.edge_visited.fill(false);
        if !self.graph.nodes.is_empty() {
            self.postorder_traversal(0);
            self.cutvalues();
        }
    }

    fn layering_topological_numbering(&mut self) {
        let num_nodes = self.graph.nodes.len();
        let mut incident = vec![0usize; num_nodes];
        // Count incoming edges using cached edge data — zero locks
        for eid in 0..self.edges.len() {
            incident[self.e_tgt_id[eid]] += 1;
        }

        let mut roots: VecDeque<usize> = VecDeque::new();
        for (nid, &inc) in incident.iter().enumerate().take(num_nodes) {
            if inc == 0 {
                roots.push_back(nid);
            }
        }

        while let Some(nid) = roots.pop_front() {
            let node_layer = self.n_layer[nid];
            // Iterate outgoing edges from cached adjacency — zero locks
            let num_adj = self.node_edges[nid].len();
            for i in 0..num_adj {
                let eid = self.node_edges[nid][i];
                if self.e_src_id[eid] != nid {
                    continue; // skip incoming edges
                }
                let tgt_id = self.e_tgt_id[eid];
                let new_layer = self.n_layer[tgt_id].max(node_layer + self.e_delta[eid]);
                self.n_layer[tgt_id] = new_layer;
                if incident[tgt_id] > 0 {
                    incident[tgt_id] -= 1;
                }
                if incident[tgt_id] == 0 {
                    roots.push_back(tgt_id);
                }
            }
        }
    }

    fn minimal_span(&self, node: &NNodeRef) -> Pair<i32, i32> {
        let mut min_span_out = i32::MAX;
        let mut min_span_in = i32::MAX;

        // Must release node lock before calling edge_target_layer/edge_source_layer,
        // because those functions lock edge→target/source which may be `node` itself
        // (incoming edges: target == node; outgoing edges: source == node).
        // parking_lot::Mutex is non-reentrant → deadlock if re-locked.
        let edges = {
            let node_guard = node.lock();
            node_guard.connected_edges()
        };

        for edge in &edges {
            let span = edge_target_layer(edge) - edge_source_layer(edge);
            let target_is_node = edge_target_is(edge, node);
            if target_is_node && span < min_span_in {
                min_span_in = span;
            } else if span < min_span_out {
                min_span_out = span;
            }
        }

        if min_span_in == i32::MAX {
            min_span_in = -1;
        }
        if min_span_out == i32::MAX {
            min_span_out = -1;
        }

        Pair::of(min_span_in, min_span_out)
    }

    fn tight_tree_dfs(&mut self, nid: usize) -> usize {
        let mut node_count = 1;
        self.n_tree_node[nid] = true;

        // Use cached adjacency list — zero Vec alloc, zero edge locks
        let num_adj = self.node_edges[nid].len();
        for i in 0..num_adj {
            let eid = self.node_edges[nid][i];
            if self.edge_visited[eid] {
                continue;
            }
            self.edge_visited[eid] = true;

            let opp_id = if self.e_src_id[eid] == nid {
                self.e_tgt_id[eid]
            } else {
                self.e_src_id[eid]
            };

            if self.e_tree[eid] {
                node_count += self.tight_tree_dfs(opp_id);
            } else if !self.n_tree_node[opp_id] {
                let src_layer = self.n_layer[self.e_src_id[eid]];
                let tgt_layer = self.n_layer[self.e_tgt_id[eid]];
                if self.e_delta[eid] == tgt_layer - src_layer {
                    self.e_tree[eid] = true;
                    self.tree_edge_ids.push(eid);
                    node_count += self.tight_tree_dfs(opp_id);
                }
            }
        }

        node_count
    }

    fn minimal_slack(&self) -> Option<usize> {
        let mut min_slack = i32::MAX;
        let mut min_eid: Option<usize> = None;
        for eid in 0..self.edges.len() {
            let src_tree = self.n_tree_node[self.e_src_id[eid]];
            let tgt_tree = self.n_tree_node[self.e_tgt_id[eid]];
            if src_tree ^ tgt_tree {
                let slack = self.n_layer[self.e_tgt_id[eid]]
                    - self.n_layer[self.e_src_id[eid]]
                    - self.e_delta[eid];
                if slack < min_slack {
                    min_slack = slack;
                    min_eid = Some(eid);
                }
            }
        }
        min_eid
    }

    fn postorder_traversal(&mut self, start_nid: usize) {
        // Iterative post-order DFS using explicit stack: (nid, adj_cursor, lowest)
        let mut stack = std::mem::take(&mut self.postorder_stack);
        stack.clear();
        stack.push((start_nid, 0, i32::MAX));

        while let Some(frame) = stack.last_mut() {
            let nid = frame.0;
            let num_adj = self.tree_edges[nid].len();

            // Advance through adjacency list to find next unvisited tree edge
            let mut pushed = false;
            while frame.1 < num_adj {
                let eid = self.tree_edges[nid][frame.1];
                frame.1 += 1;
                if self.e_tree[eid] && !self.edge_visited[eid] {
                    self.edge_visited[eid] = true;
                    let opp_id = if self.e_src_id[eid] == nid {
                        self.e_tgt_id[eid]
                    } else {
                        self.e_src_id[eid]
                    };
                    stack.push((opp_id, 0, i32::MAX));
                    pushed = true;
                    break;
                }
            }

            if !pushed {
                // Post-order: all children visited
                let (nid, _, lowest) = stack.pop().unwrap();
                let my_lowest = lowest.min(self.post_order);
                if nid < self.po_id.len() {
                    self.po_id[nid] = self.post_order;
                    self.lowest_po_id[nid] = my_lowest;
                }
                self.post_order += 1;
                // Propagate lowest to parent
                if let Some(parent) = stack.last_mut() {
                    parent.2 = parent.2.min(my_lowest);
                }
            }
        }

        self.postorder_stack = stack;
        // Fundamental-cycle cut updates require intervals for every node in
        // one spanning tree. Disconnected or source-free cyclic inputs can
        // leave nodes outside the traversal rooted at 0.
        if self.post_order as usize != self.graph.nodes.len() + 1 {
            self.integral_cutvalues = false;
        }
    }

    /// Check if `node` is in the head component of the spanning tree edge.
    /// Uses pre-extracted edge source/target internal IDs to avoid repeated locks.
    #[inline]
    fn is_in_head_by_id(&self, node_id: usize, source_id: usize, target_id: usize) -> bool {
        if self.lowest_po_id[source_id] <= self.po_id[node_id]
            && self.po_id[node_id] <= self.po_id[source_id]
            && self.lowest_po_id[target_id] <= self.po_id[node_id]
            && self.po_id[node_id] <= self.po_id[target_id]
        {
            return self.po_id[source_id] >= self.po_id[target_id];
        }
        self.po_id[source_id] < self.po_id[target_id]
    }

    fn cutvalues(&mut self) {
        let mut leafs = Vec::new();
        let mut unknown = vec![(0_usize, 0_usize); self.node_edges.len()];
        // Preserve the graph-node and incoming/outgoing edge order used by
        // leaf elimination. Only the representation changes: edge IDs replace
        // Arc handles, with the same floating-point operations in the same order.
        // Count and XOR summarize the remaining incident tree-edge IDs.
        // Only a singleton is read, whose XOR is exactly its sole edge ID.
        // Removing a determined edge updates both endpoint summaries, as the
        // reference's retain does, without allocating a list for every node.
        for (nid, pending) in unknown.iter_mut().enumerate() {
            for &eid in &self.node_edges[nid] {
                if self.e_tree[eid] {
                    pending.0 += 1;
                    pending.1 ^= eid;
                }
            }
            if pending.0 == 1 {
                leafs.push(nid);
            }
        }

        for mut node in leafs {
            loop {
                if unknown[node].0 != 1 {
                    break;
                }
                let td_id = unknown[node].1;
                if td_id >= self.cutvalue.len() {
                    break;
                }
                let td_weight = self.e_weight[td_id];
                let source = self.e_src_id[td_id];
                let target = self.e_tgt_id[td_id];
                self.cutvalue[td_id] = td_weight;

                // Use cached adjacency list for connected edges
                let nid = node;
                let num_adj = self.node_edges[nid].len();
                for i in 0..num_adj {
                    let eid = self.node_edges[nid][i];
                    if eid == td_id {
                        continue;
                    }
                    // All properties from cache — zero edge locks
                    let e_weight = self.e_weight[eid];
                    if self.e_tree[eid] {
                        let same_direction =
                            self.e_src_id[eid] == source || self.e_tgt_id[eid] == target;
                        if same_direction {
                            self.cutvalue[td_id] -= self.cutvalue[eid] - e_weight;
                        } else {
                            self.cutvalue[td_id] += self.cutvalue[eid] - e_weight;
                        }
                    } else if node == source {
                        if self.e_src_id[eid] == node {
                            self.cutvalue[td_id] += e_weight;
                        } else {
                            self.cutvalue[td_id] -= e_weight;
                        }
                    } else if self.e_src_id[eid] == node {
                        self.cutvalue[td_id] -= e_weight;
                    } else {
                        self.cutvalue[td_id] += e_weight;
                    }
                }

                for endpoint in [source, target] {
                    unknown[endpoint].0 -= 1;
                    unknown[endpoint].1 ^= td_id;
                }

                if source == node {
                    node = target;
                } else {
                    node = source;
                }
            }
        }
    }

    #[cfg(test)]
    fn cutvalues_reference(&mut self) {
        let mut leafs: Vec<NNodeRef> = Vec::new();
        for node in &self.graph.nodes {
            let mut tree_edge_count = 0;
            {
                let mut node_guard = node.lock();
                let nid = node_guard.internal_id;
                node_guard.unknown_cutvalues.clear();
                // Use cached adjacency list + e_tree to avoid per-edge locks
                for &eid in &self.node_edges[nid] {
                    if self.e_tree[eid] {
                        node_guard.unknown_cutvalues.push(self.edges[eid].clone());
                        tree_edge_count += 1;
                    }
                }
            }
            if tree_edge_count == 1 {
                leafs.push(node.clone());
            }
        }

        for mut node in leafs {
            loop {
                let to_determine = {
                    let guard = node.lock();
                    if guard.unknown_cutvalues.len() != 1 {
                        break;
                    }
                    guard.unknown_cutvalues[0].clone()
                };

                let td_id = to_determine.lock().internal_id;
                if td_id >= self.cutvalue.len() {
                    break;
                }
                // Use cached weight, source, target — zero extra locks
                let td_weight = self.e_weight[td_id];
                let source = self.e_src[td_id].clone();
                let target = self.e_tgt[td_id].clone();
                self.cutvalue[td_id] = td_weight;

                // Use cached adjacency list for connected edges
                let nid = node.lock().internal_id;
                let num_adj = self.node_edges[nid].len();
                for i in 0..num_adj {
                    let eid = self.node_edges[nid][i];
                    if eid == td_id {
                        continue;
                    }
                    // All properties from cache — zero edge locks
                    let e_weight = self.e_weight[eid];
                    if self.e_tree[eid] {
                        let same_direction = Arc::ptr_eq(&self.e_src[eid], &source)
                            || Arc::ptr_eq(&self.e_tgt[eid], &target);
                        if same_direction {
                            self.cutvalue[td_id] -= self.cutvalue[eid] - e_weight;
                        } else {
                            self.cutvalue[td_id] += self.cutvalue[eid] - e_weight;
                        }
                    } else if Arc::ptr_eq(&node, &source) {
                        if Arc::ptr_eq(&self.e_src[eid], &node) {
                            self.cutvalue[td_id] += e_weight;
                        } else {
                            self.cutvalue[td_id] -= e_weight;
                        }
                    } else if Arc::ptr_eq(&self.e_src[eid], &node) {
                        self.cutvalue[td_id] -= e_weight;
                    } else {
                        self.cutvalue[td_id] += e_weight;
                    }
                }

                remove_unknown_cutvalue(&source, &to_determine);
                remove_unknown_cutvalue(&target, &to_determine);

                if Arc::ptr_eq(&source, &node) {
                    node = target;
                } else {
                    node = source;
                }
            }
        }
    }

    fn leave_edge(&self) -> Option<usize> {
        // Must iterate tree_edge_ids in insertion order (Java parity) — zero locks
        self.tree_edge_ids
            .iter()
            .find(|&&eid| {
                self.e_tree[eid] && eid < self.cutvalue.len() && self.cutvalue[eid] < FUZZY_ST_ZERO
            })
            .copied()
    }

    fn enter_edge(&self, leave_id: usize) -> Option<usize> {
        if !self.e_tree[leave_id] {
            return None;
        }
        let leave_src_id = self.e_src_id[leave_id];
        let leave_tgt_id = self.e_tgt_id[leave_id];

        // Each node's cut membership is constant for this search. Compute it
        // once, rather than repeating interval tests for every incident edge.
        let head: Vec<bool> = (0..self.n_layer.len())
            .map(|nid| self.is_in_head_by_id(nid, leave_src_id, leave_tgt_id))
            .collect();
        let mut replace: Option<usize> = None;
        let mut rep_slack = i32::MAX;
        for eid in 0..self.edges.len() {
            let src_id = self.e_src_id[eid];
            let tgt_id = self.e_tgt_id[eid];
            if head[src_id] && !head[tgt_id] {
                let slack = self.n_layer[tgt_id] - self.n_layer[src_id] - self.e_delta[eid];
                if slack < rep_slack {
                    rep_slack = slack;
                    replace = Some(eid);
                }
            }
        }

        replace
    }

    fn exchange(&mut self, leave_id: usize, enter_id: usize) {
        if !self.e_tree[leave_id] || self.e_tree[enter_id] {
            return;
        }

        if self.integral_cutvalues {
            self.update_cutvalues(leave_id, enter_id);
        }

        // Toggle tree membership — SoA cache only (no underlying edge sync needed)
        self.e_tree[leave_id] = false;
        if let Some(pos) = self.tree_edge_ids.iter().position(|&eid| eid == leave_id) {
            self.tree_edge_ids.remove(pos);
        }

        self.e_tree[enter_id] = true;
        self.tree_edge_ids.push(enter_id);
        for nid in [self.e_src_id[leave_id], self.e_tgt_id[leave_id]] {
            self.tree_edges[nid].retain(|&eid| eid != leave_id);
        }
        for nid in [self.e_src_id[enter_id], self.e_tgt_id[enter_id]] {
            // node_edges has incoming IDs followed by outgoing IDs, each in
            // increasing internal-ID order. Preserve that exact DFS order.
            let key = (self.e_src_id[enter_id] == nid, enter_id);
            let at =
                self.tree_edges[nid].partition_point(|&eid| (self.e_src_id[eid] == nid, eid) < key);
            self.tree_edges[nid].insert(at, enter_id);
        }

        // Use cached IDs and layers — zero locks for leave/enter properties
        let leave_src_id = self.e_src_id[leave_id];
        let leave_tgt_id = self.e_tgt_id[leave_id];
        let enter_src_id = self.e_src_id[enter_id];
        let enter_tgt_id = self.e_tgt_id[enter_id];

        let mut delta =
            self.n_layer[enter_tgt_id] - self.n_layer[enter_src_id] - self.e_delta[enter_id];
        if !self.is_in_head_by_id(enter_tgt_id, leave_src_id, leave_tgt_id) {
            delta = -delta;
        }

        for nid in 0..self.n_layer.len() {
            if !self.is_in_head_by_id(nid, leave_src_id, leave_tgt_id) {
                self.n_layer[nid] += delta;
            }
        }

        self.post_order = 1;
        self.edge_visited.fill(false);
        if !self.graph.nodes.is_empty() {
            self.postorder_traversal(0);
        }
        if !self.integral_cutvalues {
            self.cutvalues();
        }
    }

    /// Only cuts on the entering edge's fundamental cycle change. Its
    /// coefficient in the leaving cut is -1 (enter_edge crosses head to tail),
    /// so the new cut is the old cut plus that cycle's signed leaving cut.
    /// Read the old tree's intervals before changing membership/postorder.
    fn update_cutvalues(&mut self, leave_id: usize, enter_id: usize) {
        let cut = self.cutvalue[leave_id];
        let source = self.e_src_id[enter_id];
        let target = self.e_tgt_id[enter_id];
        for &eid in &self.tree_edge_ids {
            if eid == leave_id {
                continue;
            }
            let source_in_head =
                self.is_in_head_by_id(source, self.e_src_id[eid], self.e_tgt_id[eid]);
            let target_in_head =
                self.is_in_head_by_id(target, self.e_src_id[eid], self.e_tgt_id[eid]);
            if source_in_head != target_in_head {
                if target_in_head {
                    self.cutvalue[eid] += cut;
                } else {
                    self.cutvalue[eid] -= cut;
                }
            }
        }
        self.cutvalue[enter_id] = -cut;
    }

    fn normalize(&mut self) -> Vec<i32> {
        let mut highest = i32::MIN;
        let mut lowest = i32::MAX;
        for node in &self.graph.nodes {
            let node_guard = node.lock();
            highest = highest.max(node_guard.layer);
            lowest = lowest.min(node_guard.layer);
        }

        let size = (highest - lowest + 1).max(1) as usize;
        let mut filling = vec![0i32; size];
        for node in &self.graph.nodes {
            let mut node_guard = node.lock();
            node_guard.layer -= lowest;
            let idx = node_guard.layer.max(0) as usize;
            if idx < filling.len() {
                filling[idx] += 1;
            }
        }

        if let Some(previous) = self.previous_layering_node_counts.as_ref() {
            for (idx, count) in previous.iter().enumerate() {
                if idx >= filling.len() {
                    break;
                }
                filling[idx] += *count;
            }
        }
        filling
    }

    fn balance_layers(&mut self, filling: &[i32]) {
        let mut filling = filling.to_vec();
        for node in &self.graph.nodes {
            let (incoming_count, outgoing_count, current_layer) = {
                let node_guard = node.lock();
                (
                    node_guard.incoming_edges().len(),
                    node_guard.outgoing_edges().len(),
                    node_guard.layer,
                )
            };

            if incoming_count != outgoing_count {
                continue;
            }

            let range = self.minimal_span(node);
            let mut new_layer = current_layer;
            for idx in (current_layer - range.first + 1)..(current_layer + range.second) {
                if idx < 0 || idx as usize >= filling.len() {
                    continue;
                }
                if filling[idx as usize] < filling[new_layer as usize] {
                    new_layer = idx;
                }
            }

            if new_layer != current_layer
                && filling[new_layer as usize] < filling[current_layer as usize]
            {
                filling[current_layer as usize] -= 1;
                filling[new_layer as usize] += 1;
                let mut node_guard = node.lock();
                node_guard.layer = new_layer;
            }
        }
    }

    fn remove_subtrees(&mut self) {
        let mut leafs: VecDeque<NNodeRef> = VecDeque::new();
        for node in &self.graph.nodes {
            let edge_count = node.lock().connected_edge_count();
            if edge_count == 1 {
                leafs.push_back(node.clone());
            }
        }

        let mut stack: VecDeque<Pair<NNodeRef, NEdgeRef>> = VecDeque::new();
        let mut removed = FxHashSet::default();
        while let Some(node) = leafs.pop_front() {
            let (edge, is_out_edge) = {
                let node_guard = node.lock();
                if node_guard.connected_edge_count() == 0 {
                    continue;
                }
                // First connected edge: incoming first, then outgoing (Java parity)
                let is_out = node_guard.incoming_edges().is_empty();
                let e = if !node_guard.incoming_edges().is_empty() {
                    node_guard.incoming_edges()[0].clone()
                } else {
                    node_guard.outgoing_edges()[0].clone()
                };
                (e, is_out)
            };
            let other = edge.lock().other(&node);
            if is_out_edge {
                remove_edge_from_node(&other, &edge, false);
            } else {
                remove_edge_from_node(&other, &edge, true);
            }
            let other_edges = other.lock().connected_edge_count();
            if other_edges == 1 {
                leafs.push_back(other);
            }
            stack.push_back(Pair::of(node.clone(), edge));
            removed.insert(Arc::as_ptr(&node) as usize);
        }
        // No pruning step reads the node vector. Compact once, preserving its
        // survivor order and the queue/stack order used for reattachment.
        self.graph
            .nodes
            .retain(|node| !removed.contains(&(Arc::as_ptr(node) as usize)));
        self.subtree_nodes_stack = Some(stack);
    }

    fn reattach_subtrees(&mut self) {
        let Some(stack) = self.subtree_nodes_stack.as_mut() else {
            return;
        };
        while let Some(pair) = stack.pop_back() {
            let node = pair.first;
            let edge = pair.second;
            let placed = edge.lock().other(&node);

            let node_is_target = edge_target_is(&edge, &node);
            if node_is_target {
                add_edge_to_node(&placed, &edge, true);
                {
                    let mut node_guard = node.lock();
                    let placed_guard = placed.lock();
                    node_guard.layer = placed_guard.layer + edge_delta(&edge);
                }
            } else {
                add_edge_to_node(&placed, &edge, false);
                {
                    let mut node_guard = node.lock();
                    let placed_guard = placed.lock();
                    node_guard.layer = placed_guard.layer - edge_delta(&edge);
                }
            }
            self.graph.nodes.push(node);
        }
    }
}

fn edge_delta(edge: &NEdgeRef) -> i32 {
    edge.lock().delta
}

fn edge_source(edge: &NEdgeRef) -> NNodeRef {
    edge.lock().source()
}

fn edge_target(edge: &NEdgeRef) -> NNodeRef {
    edge.lock().target()
}

fn edge_source_layer(edge: &NEdgeRef) -> i32 {
    edge_source(edge).lock().layer
}

fn edge_target_layer(edge: &NEdgeRef) -> i32 {
    edge_target(edge).lock().layer
}

fn edge_target_is(edge: &NEdgeRef, node: &NNodeRef) -> bool {
    Arc::ptr_eq(&edge_target(edge), node)
}

#[cfg(test)]
fn remove_unknown_cutvalue(node: &NNodeRef, edge: &NEdgeRef) {
    let mut node_guard = node.lock();
    node_guard
        .unknown_cutvalues
        .retain(|candidate| !Arc::ptr_eq(candidate, edge));
}

fn remove_edge_from_node(node: &NNodeRef, edge: &NEdgeRef, outgoing: bool) {
    let mut node_guard = node.lock();
    if outgoing {
        node_guard
            .outgoing_edges_mut()
            .retain(|candidate| !Arc::ptr_eq(candidate, edge));
    } else {
        node_guard
            .incoming_edges_mut()
            .retain(|candidate| !Arc::ptr_eq(candidate, edge));
    }
}

fn add_edge_to_node(node: &NNodeRef, edge: &NEdgeRef, outgoing: bool) {
    let mut node_guard = node.lock();
    if outgoing {
        node_guard.outgoing_edges_mut().push(edge.clone());
    } else {
        node_guard.incoming_edges_mut().push(edge.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org::eclipse::elk::alg::common::networksimplex::{NEdge, NNode};

    fn dag(count: usize, mut seed: u64, fraction: f64) -> NGraph {
        let mut graph = NGraph::new();
        for _ in 0..count {
            NNode::of().create(&mut graph);
        }
        for source in 0..count {
            for target in source + 1..count {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                if source == 0 || seed >> 60 < 2 {
                    NEdge::of()
                        .source(graph.nodes[source].clone())
                        .target(graph.nodes[target].clone())
                        .delta(1 + ((seed >> 32) % 5) as i32)
                        .weight(1.0 + (seed % 17) as f64 + fraction)
                        .create();
                }
            }
        }
        graph
    }

    #[test]
    fn indexed_fractional_cuts_match_reference_bits_at_every_pivot() {
        let mut pivots = 0;
        for count in [8, 24, 48, 80] {
            for seed in 0..24 {
                for fraction in [0.01, 0.5] {
                    let mut graph = dag(count, seed, fraction);
                    let mut simplex = NetworkSimplex::for_graph(&mut graph);
                    simplex.initialize();
                    assert!(!simplex.integral_cutvalues);
                    simplex.feasible_tree();
                    for _ in 0..512 {
                        let actual: Vec<_> = simplex.cutvalue.iter().map(|v| v.to_bits()).collect();
                        simplex.cutvalues_reference();
                        let expected: Vec<_> =
                            simplex.cutvalue.iter().map(|v| v.to_bits()).collect();
                        assert_eq!(
                            actual, expected,
                            "count={count} seed={seed} fraction={fraction}"
                        );
                        let Some(leave) = simplex.leave_edge() else {
                            break;
                        };
                        let enter = simplex.enter_edge(leave).expect("feasible pivot");
                        simplex.exchange(leave, enter);
                        pivots += 1;
                    }
                }
            }
        }
        assert!(pivots > 100, "exercise fractional tree exchanges: {pivots}");
    }

    #[test]
    fn integral_pivot_cuts_equal_full_leaf_elimination() {
        let mut pivots = 0;
        for count in [8, 24, 48, 80] {
            for seed in 0..24 {
                let mut graph = dag(count, seed, 0.0);
                let mut simplex = NetworkSimplex::for_graph(&mut graph);
                simplex.initialize();
                assert!(simplex.integral_cutvalues);
                simplex.feasible_tree();
                for _ in 0..512 {
                    let Some(leave) = simplex.leave_edge() else {
                        break;
                    };
                    let enter = simplex.enter_edge(leave).expect("feasible pivot");
                    simplex.exchange(leave, enter);
                    for nid in 0..simplex.node_edges.len() {
                        let expected: Vec<_> = simplex.node_edges[nid]
                            .iter()
                            .copied()
                            .filter(|&eid| simplex.e_tree[eid])
                            .collect();
                        assert_eq!(simplex.tree_edges[nid], expected, "same tree DFS order");
                    }
                    let incremental = simplex.cutvalue.clone();
                    simplex.cutvalues();
                    for &eid in &simplex.tree_edge_ids {
                        assert_eq!(
                            incremental[eid], simplex.cutvalue[eid],
                            "count={count} seed={seed} pivot={pivots} edge={eid}"
                        );
                    }
                    pivots += 1;
                }
            }
        }
        assert!(
            pivots > 100,
            "exercise tree exchanges, including tied cuts: {pivots}"
        );
    }

    #[test]
    fn incomplete_postorder_keeps_full_cut_computation() {
        for source_free_cycle in [false, true] {
            let mut graph = NGraph::new();
            let nodes: Vec<_> = (0..6).map(|_| NNode::of().create(&mut graph)).collect();
            let edges = if source_free_cycle {
                vec![(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]
            } else {
                vec![(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)]
            };
            for (source, target) in edges {
                NEdge::of()
                    .source(nodes[source].clone())
                    .target(nodes[target].clone())
                    .delta(1)
                    .weight(1.0)
                    .create();
            }
            let mut simplex = NetworkSimplex::for_graph(&mut graph);
            simplex.initialize();
            assert!(simplex.integral_cutvalues);
            simplex.feasible_tree();
            assert!(simplex.post_order as usize <= simplex.graph.nodes.len());
            assert!(!simplex.integral_cutvalues);
            let cuts = simplex.cutvalue.clone();
            simplex.cutvalues();
            assert_eq!(simplex.cutvalue, cuts);
        }
    }

    #[test]
    fn fractional_or_large_weights_keep_original_cut_arithmetic() {
        for fraction in [0.01, (1_u64 << 53) as f64] {
            let mut graph = dag(8, 17, fraction);
            let mut simplex = NetworkSimplex::for_graph(&mut graph);
            simplex.initialize();
            assert!(!simplex.integral_cutvalues);
        }
    }

    #[test]
    fn subtree_compaction_preserves_survivor_and_reattachment_order() {
        let mut graph = NGraph::new();
        let nodes: Vec<_> = (0..7).map(|_| NNode::of().create(&mut graph)).collect();
        for (source, target) in [(0, 1), (1, 2), (0, 2), (2, 3), (3, 4), (0, 5), (1, 6)] {
            NEdge::of()
                .source(nodes[source].clone())
                .target(nodes[target].clone())
                .delta(1)
                .create();
        }
        let mut simplex = NetworkSimplex::for_graph(&mut graph);
        simplex.remove_subtrees();
        let ordinals = |graph: &NGraph| {
            graph
                .nodes
                .iter()
                .map(|node| {
                    nodes
                        .iter()
                        .position(|original| Arc::ptr_eq(original, node))
                        .unwrap()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(ordinals(simplex.graph), [0, 1, 2]);
        nodes[1].lock().layer = 1;
        nodes[2].lock().layer = 2;
        simplex.reattach_subtrees();
        assert_eq!(ordinals(simplex.graph), [0, 1, 2, 3, 6, 5, 4]);
        assert_eq!(
            nodes
                .iter()
                .map(|node| node.lock().layer)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 1, 2]
        );
    }
}
