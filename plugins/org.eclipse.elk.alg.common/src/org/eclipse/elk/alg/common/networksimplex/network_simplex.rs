use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::LazyLock;

use rustc_hash::FxHashSet;

static TRACE_NETWORK_SIMPLEX: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_NETWORK_SIMPLEX").is_some());

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
    tree_edges: Vec<NEdgeRef>,
    tree_edge_set: FxHashSet<usize>,
    sources: Vec<NNodeRef>,
    edge_visited: Vec<bool>,
    post_order: i32,
    po_id: Vec<i32>,
    lowest_po_id: Vec<i32>,
    cutvalue: Vec<f64>,
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
}

impl<'a> NetworkSimplex<'a> {
    pub fn for_graph(graph: &'a mut NGraph) -> Self {
        NetworkSimplex {
            graph,
            previous_layering_node_counts: None,
            balance: false,
            iteration_limit: i32::MAX,
            edges: Vec::new(),
            tree_edges: Vec::new(),
            tree_edge_set: FxHashSet::default(),
            sources: Vec::new(),
            edge_visited: Vec::new(),
            post_order: 1,
            po_id: Vec::new(),
            lowest_po_id: Vec::new(),
            cutvalue: Vec::new(),
            subtree_nodes_stack: None,
            e_src: Vec::new(),
            e_tgt: Vec::new(),
            e_src_id: Vec::new(),
            e_tgt_id: Vec::new(),
            e_delta: Vec::new(),
            e_weight: Vec::new(),
            e_tree: Vec::new(),
            node_edges: Vec::new(),
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
            if let Ok(mut node_guard) = node.lock() {
                node_guard.layer = 0;
            }
        }

        let remove_subtrees = self.graph.nodes.len() >= REMOVE_SUBTREES_THRESH;
        if remove_subtrees {
            self.remove_subtrees();
        }

        self.initialize();
        self.feasible_tree();

        let mut edge = self.leave_edge();
        let mut iter = 0;
        while let Some(leave) = edge {
            if iter >= self.iteration_limit {
                break;
            }
            if let Some(enter) = self.enter_edge(&leave) {
                self.exchange(&leave, &enter);
            } else {
                if *TRACE_NETWORK_SIMPLEX {
                    eprintln!(
                        "[network-simplex] break: missing entering edge at iter={iter} leave_edge_id={}",
                        edge_internal_id(&leave)
                    );
                }
                break;
            }
            edge = self.leave_edge();
            iter += 1;
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
        for node in &self.graph.nodes {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.tree_node = false;
            }
        }
        self.po_id = vec![0; num_nodes];
        self.lowest_po_id = vec![0; num_nodes];
        self.sources.clear();

        let mut edges = Vec::new();
        for (index, node) in self.graph.nodes.iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.internal_id = index;
                if node_guard.incoming_edges().is_empty() {
                    self.sources.push(node.clone());
                }
                edges.extend(node_guard.outgoing_edges().clone());
            }
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
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.internal_id = index;
                edge_guard.tree_edge = false;
                self.e_src.push(edge_guard.source.clone());
                self.e_tgt.push(edge_guard.target.clone());
                self.e_delta[index] = edge_guard.delta;
                self.e_weight[index] = edge_guard.weight;
                self.e_tree[index] = false;
                // source/target internal_ids are already set above
                let src_id = edge_guard
                    .source
                    .lock()
                    .map(|g| g.internal_id)
                    .unwrap_or(0);
                let tgt_id = edge_guard
                    .target
                    .lock()
                    .map(|g| g.internal_id)
                    .unwrap_or(0);
                self.e_src_id[index] = src_id;
                self.e_tgt_id[index] = tgt_id;
            } else {
                // Fallback: push empty refs (shouldn't happen)
                self.e_src.push(self.graph.nodes[0].clone());
                self.e_tgt.push(self.graph.nodes[0].clone());
            }
        }

        // Build per-node adjacency list (incoming first, then outgoing — Java parity)
        self.node_edges.clear();
        self.node_edges.resize(num_nodes, Vec::new());
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
        self.tree_edges.clear();
        self.tree_edge_set.clear();
        self.post_order = 1;
    }

    fn dispose(&mut self) {
        self.cutvalue.clear();
        self.edges.clear();
        self.tree_edges.clear();
        self.tree_edge_set.clear();
        self.edge_visited.clear();
        self.e_src.clear();
        self.e_tgt.clear();
        self.e_src_id.clear();
        self.e_tgt_id.clear();
        self.e_delta.clear();
        self.e_weight.clear();
        self.e_tree.clear();
        self.node_edges.clear();
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
        while let Some(root) = self.graph.nodes.first().cloned() {
            let count = self.tight_tree_dfs(&root);
            if count >= self.graph.nodes.len() {
                break;
            }
            let Some(edge) = self.minimal_slack() else {
                break;
            };
            let (e_delta, e_src, e_tgt) = match edge.lock() {
                Ok(g) => (g.delta, g.source.clone(), g.target.clone()),
                Err(_) => break,
            };
            let src_layer = e_src.lock().map(|g| g.layer).unwrap_or(0);
            let tgt_layer = e_tgt.lock().map(|g| g.layer).unwrap_or(0);
            let tgt_tree = e_tgt.lock().map(|g| g.tree_node).unwrap_or(false);
            let mut slack = tgt_layer - src_layer - e_delta;
            if tgt_tree {
                slack = -slack;
            }
            for node in &self.graph.nodes {
                if let Ok(mut node_guard) = node.lock() {
                    if node_guard.tree_node {
                        node_guard.layer += slack;
                    }
                }
            }
            self.edge_visited.fill(false);
        }

        self.edge_visited.fill(false);
        if let Some(root) = self.graph.nodes.first().cloned() {
            self.postorder_traversal(&root);
            self.cutvalues();
        }
    }

    fn layering_topological_numbering(&mut self) {
        let mut incident = vec![0usize; self.graph.nodes.len()];
        for node in &self.graph.nodes {
            let idx = node_internal_id(node);
            if let Ok(node_guard) = node.lock() {
                incident[idx] += node_guard.incoming_edges().len();
            }
        }

        let mut roots: VecDeque<NNodeRef> = self.sources.iter().cloned().collect();
        while let Some(node) = roots.pop_front() {
            let (node_layer, outgoing) = match node.lock() {
                Ok(node_guard) => (node_guard.layer, node_guard.outgoing_edges().clone()),
                Err(_) => (0, Vec::new()),
            };
            for edge in outgoing {
                let target = edge.lock().ok().map(|edge_guard| edge_guard.target.clone());
                let Some(target) = target else {
                    continue;
                };
                if let Ok(mut target_guard) = target.lock() {
                    target_guard.layer = target_guard.layer.max(node_layer + edge_delta(&edge));
                }
                let target_idx = node_internal_id(&target);
                if incident[target_idx] > 0 {
                    incident[target_idx] -= 1;
                }
                if incident[target_idx] == 0 {
                    roots.push_back(target);
                }
            }
        }
    }

    fn minimal_span(&self, node: &NNodeRef) -> Pair<i32, i32> {
        let mut min_span_out = i32::MAX;
        let mut min_span_in = i32::MAX;

        let edges = match node.lock() {
            Ok(node_guard) => node_guard.connected_edges(),
            Err(_) => Vec::new(),
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

    fn tight_tree_dfs(&mut self, node: &NNodeRef) -> usize {
        let mut node_count = 1;
        let nid = node.lock().map(|mut g| { g.tree_node = true; g.internal_id }).unwrap_or(0);

        // Use cached adjacency list — zero Vec alloc, zero edge locks
        let num_adj = self.node_edges[nid].len();
        for i in 0..num_adj {
            let eid = self.node_edges[nid][i];
            if self.edge_visited[eid] {
                continue;
            }
            self.edge_visited[eid] = true;

            let is_tree = self.e_tree[eid];
            let opposite = if self.e_src_id[eid] == nid {
                self.e_tgt[eid].clone()
            } else {
                self.e_src[eid].clone()
            };

            if is_tree {
                node_count += self.tight_tree_dfs(&opposite);
            } else {
                let opp_tree = opposite.lock().map(|g| g.tree_node).unwrap_or(false);
                if !opp_tree {
                    let src_layer = self.e_src[eid].lock().map(|g| g.layer).unwrap_or(0);
                    let tgt_layer = self.e_tgt[eid].lock().map(|g| g.layer).unwrap_or(0);
                    if self.e_delta[eid] == tgt_layer - src_layer {
                        self.e_tree[eid] = true;
                        if let Ok(mut edge_guard) = self.edges[eid].lock() {
                            edge_guard.tree_edge = true;
                        }
                        let edge_ptr = Arc::as_ptr(&self.edges[eid]) as usize;
                        if self.tree_edge_set.insert(edge_ptr) {
                            self.tree_edges.push(self.edges[eid].clone());
                        }
                        node_count += self.tight_tree_dfs(&opposite);
                    }
                }
            }
        }

        node_count
    }

    fn minimal_slack(&self) -> Option<NEdgeRef> {
        let mut min_slack = i32::MAX;
        let mut min_edge: Option<NEdgeRef> = None;
        for eid in 0..self.edges.len() {
            let src_tree = self.e_src[eid].lock().map(|g| g.tree_node).unwrap_or(false);
            let tgt_tree = self.e_tgt[eid].lock().map(|g| g.tree_node).unwrap_or(false);
            if src_tree ^ tgt_tree {
                let src_layer = self.e_src[eid].lock().map(|g| g.layer).unwrap_or(0);
                let tgt_layer = self.e_tgt[eid].lock().map(|g| g.layer).unwrap_or(0);
                let slack = tgt_layer - src_layer - self.e_delta[eid];
                if slack < min_slack {
                    min_slack = slack;
                    min_edge = Some(self.edges[eid].clone());
                }
            }
        }
        min_edge
    }

    fn postorder_traversal(&mut self, node: &NNodeRef) -> i32 {
        let mut lowest = i32::MAX;
        let nid = node.lock().map(|g| g.internal_id).unwrap_or(0);
        let num_adj = self.node_edges[nid].len();
        for i in 0..num_adj {
            let eid = self.node_edges[nid][i];
            if self.e_tree[eid] && !self.edge_visited[eid] {
                self.edge_visited[eid] = true;
                let other = if self.e_src_id[eid] == nid {
                    self.e_tgt[eid].clone()
                } else {
                    self.e_src[eid].clone()
                };
                lowest = lowest.min(self.postorder_traversal(&other));
            }
        }

        let node_id = nid;
        if node_id < self.po_id.len() {
            self.po_id[node_id] = self.post_order;
            self.lowest_po_id[node_id] = lowest.min(self.post_order);
        }
        self.post_order += 1;
        self.lowest_po_id
            .get(node_id)
            .copied()
            .unwrap_or(self.post_order)
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
        let mut leafs: Vec<NNodeRef> = Vec::new();
        for node in &self.graph.nodes {
            let mut tree_edge_count = 0;
            if let Ok(mut node_guard) = node.lock() {
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
                    let guard = node.lock().ok();
                    let Some(guard) = guard else {
                        break;
                    };
                    if guard.unknown_cutvalues.len() != 1 {
                        break;
                    }
                    guard.unknown_cutvalues[0].clone()
                };

                let td_id = to_determine.lock().map(|g| g.internal_id).unwrap_or(0);
                if td_id >= self.cutvalue.len() {
                    break;
                }
                // Use cached weight, source, target — zero extra locks
                let td_weight = self.e_weight[td_id];
                let source = self.e_src[td_id].clone();
                let target = self.e_tgt[td_id].clone();
                self.cutvalue[td_id] = td_weight;

                // Use cached adjacency list for connected edges
                let nid = node.lock().map(|g| g.internal_id).unwrap_or(0);
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

    fn leave_edge(&self) -> Option<NEdgeRef> {
        for edge in &self.tree_edges {
            let eid = edge.lock().map(|g| g.internal_id).unwrap_or(0);
            if self.e_tree[eid] && eid < self.cutvalue.len() && self.cutvalue[eid] < FUZZY_ST_ZERO {
                return Some(edge.clone());
            }
        }
        None
    }

    fn enter_edge(&self, leave: &NEdgeRef) -> Option<NEdgeRef> {
        let leave_id = leave.lock().map(|g| g.internal_id).unwrap_or(0);
        if !self.e_tree[leave_id] {
            return None;
        }
        let leave_src_id = self.e_src_id[leave_id];
        let leave_tgt_id = self.e_tgt_id[leave_id];

        let mut replace: Option<NEdgeRef> = None;
        let mut rep_slack = i32::MAX;
        for eid in 0..self.edges.len() {
            let src_id = self.e_src_id[eid];
            let tgt_id = self.e_tgt_id[eid];
            if self.is_in_head_by_id(src_id, leave_src_id, leave_tgt_id)
                && !self.is_in_head_by_id(tgt_id, leave_src_id, leave_tgt_id)
            {
                let src_layer = self.e_src[eid].lock().map(|g| g.layer).unwrap_or(0);
                let tgt_layer = self.e_tgt[eid].lock().map(|g| g.layer).unwrap_or(0);
                let slack = tgt_layer - src_layer - self.e_delta[eid];
                if slack < rep_slack {
                    rep_slack = slack;
                    replace = Some(self.edges[eid].clone());
                }
            }
        }

        replace
    }

    fn exchange(&mut self, leave: &NEdgeRef, enter: &NEdgeRef) {
        let leave_id = leave.lock().map(|g| g.internal_id).unwrap_or(0);
        let enter_id = enter.lock().map(|g| g.internal_id).unwrap_or(0);

        if !self.e_tree[leave_id] || self.e_tree[enter_id] {
            return;
        }

        // Toggle tree_edge on both edges — sync cache + underlying edge
        if let Ok(mut edge_guard) = leave.lock() {
            edge_guard.tree_edge = false;
        }
        self.e_tree[leave_id] = false;
        let leave_ptr = Arc::as_ptr(leave) as usize;
        self.tree_edge_set.remove(&leave_ptr);
        self.tree_edges.retain(|edge| !Arc::ptr_eq(edge, leave));

        if let Ok(mut edge_guard) = enter.lock() {
            edge_guard.tree_edge = true;
        }
        self.e_tree[enter_id] = true;
        let enter_ptr = Arc::as_ptr(enter) as usize;
        if self.tree_edge_set.insert(enter_ptr) {
            self.tree_edges.push(enter.clone());
        }

        // Use cached IDs — zero locks for leave/enter properties
        let leave_src_id = self.e_src_id[leave_id];
        let leave_tgt_id = self.e_tgt_id[leave_id];
        let enter_tgt_id = self.e_tgt_id[enter_id];
        let enter_src_layer = self.e_src[enter_id].lock().map(|g| g.layer).unwrap_or(0);
        let enter_tgt_layer = self.e_tgt[enter_id].lock().map(|g| g.layer).unwrap_or(0);

        let mut delta = enter_tgt_layer - enter_src_layer - self.e_delta[enter_id];
        if !self.is_in_head_by_id(enter_tgt_id, leave_src_id, leave_tgt_id) {
            delta = -delta;
        }

        for node in &self.graph.nodes {
            let nid = node.lock().map(|g| g.internal_id).unwrap_or(0);
            if !self.is_in_head_by_id(nid, leave_src_id, leave_tgt_id) {
                if let Ok(mut node_guard) = node.lock() {
                    node_guard.layer += delta;
                }
            }
        }

        self.post_order = 1;
        self.edge_visited.fill(false);
        if let Some(root) = self.graph.nodes.first().cloned() {
            self.postorder_traversal(&root);
        }
        self.cutvalues();
    }

    fn normalize(&mut self) -> Vec<i32> {
        let mut highest = i32::MIN;
        let mut lowest = i32::MAX;
        for node in &self.graph.nodes {
            if let Ok(node_guard) = node.lock() {
                highest = highest.max(node_guard.layer);
                lowest = lowest.min(node_guard.layer);
            }
        }

        let size = (highest - lowest + 1).max(1) as usize;
        let mut filling = vec![0i32; size];
        for node in &self.graph.nodes {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.layer -= lowest;
                let idx = node_guard.layer.max(0) as usize;
                if idx < filling.len() {
                    filling[idx] += 1;
                }
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
            let (incoming_count, outgoing_count, current_layer) = match node.lock() {
                Ok(node_guard) => (
                    node_guard.incoming_edges().len(),
                    node_guard.outgoing_edges().len(),
                    node_guard.layer,
                ),
                Err(_) => continue,
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
                if let Ok(mut node_guard) = node.lock() {
                    node_guard.layer = new_layer;
                }
            }
        }
    }

    fn remove_subtrees(&mut self) {
        let mut leafs: VecDeque<NNodeRef> = VecDeque::new();
        for node in &self.graph.nodes {
            let edge_count = node
                .lock()
                .map(|guard| guard.connected_edge_count())
                .unwrap_or(0);
            if edge_count == 1 {
                leafs.push_back(node.clone());
            }
        }

        let mut stack: VecDeque<Pair<NNodeRef, NEdgeRef>> = VecDeque::new();
        while let Some(node) = leafs.pop_front() {
            let (edge, is_out_edge) = match node.lock() {
                Ok(node_guard) => {
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
                }
                Err(_) => continue,
            };
            let other = edge.lock().ok().map(|edge_guard| edge_guard.other(&node));
            let Some(other) = other else {
                continue;
            };
            if is_out_edge {
                remove_edge_from_node(&other, &edge, false);
            } else {
                remove_edge_from_node(&other, &edge, true);
            }
            let other_edges = other
                .lock()
                .map(|guard| guard.connected_edge_count())
                .unwrap_or(0);
            if other_edges == 1 {
                leafs.push_back(other);
            }
            stack.push_back(Pair::of(node.clone(), edge));
            self.graph
                .nodes
                .retain(|candidate| !Arc::ptr_eq(candidate, &node));
        }
        self.subtree_nodes_stack = Some(stack);
    }

    fn reattach_subtrees(&mut self) {
        let Some(stack) = self.subtree_nodes_stack.as_mut() else {
            return;
        };
        while let Some(pair) = stack.pop_back() {
            let node = pair.first;
            let edge = pair.second;
            let placed = edge.lock().ok().map(|edge_guard| edge_guard.other(&node));
            let Some(placed) = placed else {
                continue;
            };

            let node_is_target = edge_target_is(&edge, &node);
            if node_is_target {
                add_edge_to_node(&placed, &edge, true);
                if let (Ok(mut node_guard), Ok(placed_guard)) = (node.lock(), placed.lock()) {
                    node_guard.layer = placed_guard.layer + edge_delta(&edge);
                }
            } else {
                add_edge_to_node(&placed, &edge, false);
                if let (Ok(mut node_guard), Ok(placed_guard)) = (node.lock(), placed.lock()) {
                    node_guard.layer = placed_guard.layer - edge_delta(&edge);
                }
            }
            self.graph.nodes.push(node);
        }
    }
}

fn node_internal_id(node: &NNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|node_guard| node_guard.internal_id)
        .unwrap_or(0)
}

fn edge_internal_id(edge: &NEdgeRef) -> usize {
    edge.lock().map(|guard| guard.internal_id).unwrap_or(0)
}

fn edge_delta(edge: &NEdgeRef) -> i32 {
    edge.lock().map(|guard| guard.delta).unwrap_or(0)
}

fn edge_source(edge: &NEdgeRef) -> NNodeRef {
    edge.lock().map(|guard| guard.source.clone()).unwrap()
}

fn edge_target(edge: &NEdgeRef) -> NNodeRef {
    edge.lock().map(|guard| guard.target.clone()).unwrap()
}

fn edge_source_layer(edge: &NEdgeRef) -> i32 {
    edge_source(edge)
        .lock()
        .map(|guard| guard.layer)
        .unwrap_or(0)
}

fn edge_target_layer(edge: &NEdgeRef) -> i32 {
    edge_target(edge)
        .lock()
        .map(|guard| guard.layer)
        .unwrap_or(0)
}

fn edge_target_is(edge: &NEdgeRef, node: &NNodeRef) -> bool {
    Arc::ptr_eq(&edge_target(edge), node)
}

fn remove_unknown_cutvalue(node: &NNodeRef, edge: &NEdgeRef) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard
            .unknown_cutvalues
            .retain(|candidate| !Arc::ptr_eq(candidate, edge));
    }
}

fn remove_edge_from_node(node: &NNodeRef, edge: &NEdgeRef, outgoing: bool) {
    if let Ok(mut node_guard) = node.lock() {
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
}

fn add_edge_to_node(node: &NNodeRef, edge: &NEdgeRef, outgoing: bool) {
    if let Ok(mut node_guard) = node.lock() {
        if outgoing {
            node_guard.outgoing_edges_mut().push(edge.clone());
        } else {
            node_guard.incoming_edges_mut().push(edge.clone());
        }
    }
}
