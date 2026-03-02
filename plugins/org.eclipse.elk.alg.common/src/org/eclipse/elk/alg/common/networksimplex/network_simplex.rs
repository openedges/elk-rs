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

        for (index, edge) in edges.iter().enumerate() {
            if let Ok(mut edge_guard) = edge.lock() {
                edge_guard.internal_id = index;
                edge_guard.tree_edge = false;
            }
        }

        let num_edges = edges.len();
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
        if let Ok(mut node_guard) = node.lock() {
            node_guard.tree_node = true;
        }

        let edges = match node.lock() {
            Ok(node_guard) => node_guard.connected_edges(),
            Err(_) => Vec::new(),
        };

        for edge in &edges {
            // Batch: single edge lock to extract all needed properties
            let (edge_id, is_tree, delta, src, tgt) = match edge.lock() {
                Ok(g) => (
                    g.internal_id,
                    g.tree_edge,
                    g.delta,
                    g.source.clone(),
                    g.target.clone(),
                ),
                Err(_) => continue,
            };
            if self.edge_visited.get(edge_id).copied().unwrap_or(false) {
                continue;
            }
            if let Some(flag) = self.edge_visited.get_mut(edge_id) {
                *flag = true;
            }
            let opposite = if Arc::ptr_eq(&src, node) {
                &tgt
            } else {
                &src
            };

            if is_tree {
                node_count += self.tight_tree_dfs(opposite);
            } else {
                let opp_tree = opposite.lock().map(|g| g.tree_node).unwrap_or(false);
                if !opp_tree {
                    let src_layer = src.lock().map(|g| g.layer).unwrap_or(0);
                    let tgt_layer = tgt.lock().map(|g| g.layer).unwrap_or(0);
                    if delta == tgt_layer - src_layer {
                        if let Ok(mut edge_guard) = edge.lock() {
                            edge_guard.tree_edge = true;
                        }
                        let edge_ptr = Arc::as_ptr(edge) as usize;
                        if self.tree_edge_set.insert(edge_ptr) {
                            self.tree_edges.push(edge.clone());
                        }
                        node_count += self.tight_tree_dfs(opposite);
                    }
                }
            }
        }

        node_count
    }

    fn minimal_slack(&self) -> Option<NEdgeRef> {
        let mut min_slack = i32::MAX;
        let mut min_edge: Option<NEdgeRef> = None;
        for edge in &self.edges {
            // Batch: single edge lock to get source/target + delta
            let (delta, src, tgt) = match edge.lock() {
                Ok(g) => (g.delta, g.source.clone(), g.target.clone()),
                Err(_) => continue,
            };
            let src_tree = src.lock().map(|g| g.tree_node).unwrap_or(false);
            let tgt_tree = tgt.lock().map(|g| g.tree_node).unwrap_or(false);
            if src_tree ^ tgt_tree {
                let src_layer = src.lock().map(|g| g.layer).unwrap_or(0);
                let tgt_layer = tgt.lock().map(|g| g.layer).unwrap_or(0);
                let slack = tgt_layer - src_layer - delta;
                if slack < min_slack {
                    min_slack = slack;
                    min_edge = Some(edge.clone());
                }
            }
        }
        min_edge
    }

    fn postorder_traversal(&mut self, node: &NNodeRef) -> i32 {
        let mut lowest = i32::MAX;
        let edges = match node.lock() {
            Ok(node_guard) => node_guard.connected_edges(),
            Err(_) => Vec::new(),
        };
        for edge in &edges {
            // Batch: single edge lock
            let (edge_id, is_tree, src, tgt) = match edge.lock() {
                Ok(g) => (g.internal_id, g.tree_edge, g.source.clone(), g.target.clone()),
                Err(_) => continue,
            };
            if is_tree && !self.edge_visited.get(edge_id).copied().unwrap_or(false) {
                if let Some(flag) = self.edge_visited.get_mut(edge_id) {
                    *flag = true;
                }
                let other = if Arc::ptr_eq(&src, node) { tgt } else { src };
                lowest = lowest.min(self.postorder_traversal(&other));
            }
        }

        let node_id = node_internal_id(node);
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
                node_guard.unknown_cutvalues.clear();
                // Collect tree edges into unknown_cutvalues; iterate incoming then outgoing.
                for i in 0..node_guard.incoming_edges().len() {
                    let edge = node_guard.incoming_edges()[i].clone();
                    if edge_is_tree_edge(&edge) {
                        node_guard.unknown_cutvalues.push(edge);
                        tree_edge_count += 1;
                    }
                }
                for i in 0..node_guard.outgoing_edges().len() {
                    let edge = node_guard.outgoing_edges()[i].clone();
                    if edge_is_tree_edge(&edge) {
                        node_guard.unknown_cutvalues.push(edge);
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

                // Batch: single lock on to_determine for all properties
                let (edge_id, td_weight, source, target) = match to_determine.lock() {
                    Ok(g) => (g.internal_id, g.weight, g.source.clone(), g.target.clone()),
                    Err(_) => break,
                };
                if edge_id >= self.cutvalue.len() {
                    break;
                }
                self.cutvalue[edge_id] = td_weight;

                let cv_edges = match node.lock() {
                    Ok(node_guard) => node_guard.connected_edges(),
                    Err(_) => Vec::new(),
                };

                for edge in &cv_edges {
                    if Arc::ptr_eq(edge, &to_determine) {
                        continue;
                    }
                    // Batch: single lock per inner edge
                    let (e_id, e_tree, e_weight, e_src, e_tgt) = match edge.lock() {
                        Ok(g) => (
                            g.internal_id,
                            g.tree_edge,
                            g.weight,
                            g.source.clone(),
                            g.target.clone(),
                        ),
                        Err(_) => continue,
                    };
                    if e_tree {
                        let same_direction =
                            Arc::ptr_eq(&e_src, &source) || Arc::ptr_eq(&e_tgt, &target);
                        if same_direction {
                            self.cutvalue[edge_id] -= self.cutvalue[e_id] - e_weight;
                        } else {
                            self.cutvalue[edge_id] += self.cutvalue[e_id] - e_weight;
                        }
                    } else if Arc::ptr_eq(&node, &source) {
                        if Arc::ptr_eq(&e_src, &node) {
                            self.cutvalue[edge_id] += e_weight;
                        } else {
                            self.cutvalue[edge_id] -= e_weight;
                        }
                    } else if Arc::ptr_eq(&e_src, &node) {
                        self.cutvalue[edge_id] -= e_weight;
                    } else {
                        self.cutvalue[edge_id] += e_weight;
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
            // Batch: single edge lock
            let (is_tree, id) = match edge.lock() {
                Ok(g) => (g.tree_edge, g.internal_id),
                Err(_) => continue,
            };
            if is_tree && id < self.cutvalue.len() && self.cutvalue[id] < FUZZY_ST_ZERO {
                return Some(edge.clone());
            }
        }
        None
    }

    fn enter_edge(&self, leave: &NEdgeRef) -> Option<NEdgeRef> {
        // Pre-extract leave edge properties once (3 locks total instead of 5 per iteration)
        let (leave_is_tree, leave_src, leave_tgt) = match leave.lock() {
            Ok(g) => (g.tree_edge, g.source.clone(), g.target.clone()),
            Err(_) => return None,
        };
        if !leave_is_tree {
            return None;
        }
        let leave_src_id = leave_src.lock().map(|g| g.internal_id).unwrap_or(0);
        let leave_tgt_id = leave_tgt.lock().map(|g| g.internal_id).unwrap_or(0);

        let mut replace: Option<NEdgeRef> = None;
        let mut rep_slack = i32::MAX;
        for edge in &self.edges {
            // Batch: single edge lock
            let (delta, src, tgt) = match edge.lock() {
                Ok(g) => (g.delta, g.source.clone(), g.target.clone()),
                Err(_) => continue,
            };
            let src_id = src.lock().map(|g| g.internal_id).unwrap_or(0);
            let tgt_id = tgt.lock().map(|g| g.internal_id).unwrap_or(0);
            if self.is_in_head_by_id(src_id, leave_src_id, leave_tgt_id)
                && !self.is_in_head_by_id(tgt_id, leave_src_id, leave_tgt_id)
            {
                let src_layer = src.lock().map(|g| g.layer).unwrap_or(0);
                let tgt_layer = tgt.lock().map(|g| g.layer).unwrap_or(0);
                let slack = tgt_layer - src_layer - delta;
                if slack < rep_slack {
                    rep_slack = slack;
                    replace = Some(edge.clone());
                }
            }
        }

        replace
    }

    fn exchange(&mut self, leave: &NEdgeRef, enter: &NEdgeRef) {
        // Batch: extract leave/enter properties with minimal locks
        let (leave_is_tree, leave_src_id, leave_tgt_id) = {
            let lg = match leave.lock() {
                Ok(g) => (g.tree_edge, g.source.clone(), g.target.clone()),
                Err(_) => return,
            };
            let src_id = lg.1.lock().map(|g| g.internal_id).unwrap_or(0);
            let tgt_id = lg.2.lock().map(|g| g.internal_id).unwrap_or(0);
            (lg.0, src_id, tgt_id)
        };
        let enter_is_tree = enter.lock().map(|g| g.tree_edge).unwrap_or(false);

        if !leave_is_tree || enter_is_tree {
            return;
        }

        {
            if let Ok(mut edge_guard) = leave.lock() {
                edge_guard.tree_edge = false;
            }
        }
        let leave_ptr = Arc::as_ptr(leave) as usize;
        self.tree_edge_set.remove(&leave_ptr);
        self.tree_edges.retain(|edge| !Arc::ptr_eq(edge, leave));
        {
            if let Ok(mut edge_guard) = enter.lock() {
                edge_guard.tree_edge = true;
            }
        }
        let enter_ptr = Arc::as_ptr(enter) as usize;
        if self.tree_edge_set.insert(enter_ptr) {
            self.tree_edges.push(enter.clone());
        }

        // Batch: enter edge properties in single lock
        let (enter_delta, enter_src, enter_tgt) = match enter.lock() {
            Ok(g) => (g.delta, g.source.clone(), g.target.clone()),
            Err(_) => return,
        };
        let enter_src_layer = enter_src.lock().map(|g| g.layer).unwrap_or(0);
        let enter_tgt_layer = enter_tgt.lock().map(|g| g.layer).unwrap_or(0);
        let enter_tgt_id = enter_tgt.lock().map(|g| g.internal_id).unwrap_or(0);

        let mut delta = enter_tgt_layer - enter_src_layer - enter_delta;
        if !self.is_in_head_by_id(enter_tgt_id, leave_src_id, leave_tgt_id) {
            delta = -delta;
        }
        // Pre-extracted leave_src_id/leave_tgt_id: avoids 5 locks per node in loop
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

fn edge_is_tree_edge(edge: &NEdgeRef) -> bool {
    edge.lock().map(|guard| guard.tree_edge).unwrap_or(false)
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
