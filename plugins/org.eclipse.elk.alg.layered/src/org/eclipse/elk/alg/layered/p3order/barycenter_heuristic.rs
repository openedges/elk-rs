use std::any::Any;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

static TRACE_PORT_RANKS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_PORT_RANKS").is_some());
static TRACE_CROSSMIN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN").is_some());
static TRACE_BARYCENTER_NAN: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_BARYCENTER_NAN").is_some());
static TRACE_CROSSMIN_TIMING: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("ELK_TRACE_CROSSMIN_TIMING").is_some());
static TRACE_BARYCENTER_LAYER_PATTERN: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("ELK_TRACE_BARYCENTER_LAYER_PATTERN").ok());

use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
use crate::org::eclipse::elk::alg::layered::p3order::random_trace;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, PortType};
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_port_distributor::BarycenterPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::cross_min_snapshot::CrossMinSnapshot;
use crate::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;
use crate::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;

pub struct BarycenterHeuristic {
    pub(crate) port_ranks: Vec<f64>,
    pub(crate) constraint_resolver: ForsterConstraintResolver,
    pub(crate) port_distributor: Box<dyn BarycenterPortDistributor>,
    pub sweep_iteration: usize,
    snapshot: Option<Arc<CrossMinSnapshot>>,
}

impl BarycenterHeuristic {
    pub fn new(
        constraint_resolver: ForsterConstraintResolver,
        port_distributor: Box<dyn BarycenterPortDistributor>,
    ) -> Self {
        BarycenterHeuristic {
            port_ranks: Vec::new(),
            constraint_resolver,
            port_distributor,
            sweep_iteration: 0,
            snapshot: None,
        }
    }

    pub fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.port_distributor.set_snapshot(snapshot.clone());
        self.snapshot = Some(snapshot);
    }

    #[inline]
    fn bary_state(&self, li: usize, ni: usize) -> Option<&BarycenterState> {
        self.constraint_resolver.barycenter_states
            .get(li).and_then(|l| l.get(ni)).and_then(|o| o.as_ref())
    }

    #[inline]
    fn bary_state_mut(&mut self, li: usize, ni: usize) -> Option<&mut BarycenterState> {
        self.constraint_resolver.barycenter_states
            .get_mut(li).and_then(|l| l.get_mut(ni)).and_then(|o| o.as_mut())
    }

    pub(crate) fn get_barycenter(&self, node: &LNodeRef) -> Option<f64> {
        let li = self.layer_index_of(node);
        let ni = self.node_id_of(node);
        self.bary_state(li, ni).and_then(|s| s.barycenter)
    }

    #[inline]
    fn port_id_of(&self, port: &LPortRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.port_id(port) as usize
        } else {
            port_id(port)
        }
    }

    #[inline]
    fn node_id_of(&self, node: &LNodeRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.node_id(node) as usize
        } else {
            node_id(node)
        }
    }

    #[inline]
    fn layer_index_of(&self, node: &LNodeRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.node_layer_index(node) as usize
        } else {
            layer_index(node)
        }
    }

    #[inline]
    fn same_layer_check(&self, a: &LNodeRef, b: &LNodeRef) -> bool {
        if let Some(ref snap) = self.snapshot {
            snap.same_layer(a, b)
        } else {
            same_layer(a, b)
        }
    }

    pub(crate) fn randomize_barycenters(&mut self, nodes: &[LNodeRef], random: &mut Random) {
        for node in nodes {
            let node_label = node
                .lock()
                .ok()
                .map(|mut g| format!("id:{}", g.shape().graph_element().id))
                .unwrap_or_else(|| "<locked>".into());
            let raw = random.next_double();
            let value = random_trace::trace_next_double(
                raw,
                &format!("barycenter::randomize_barycenters node={node_label}"),
            );
            let li = self.layer_index_of(node);
            let ni = self.node_id_of(node);
            if let Some(state) = self.bary_state_mut(li, ni) {
                state.barycenter = Some(value);
                state.summed_weight = value;
                state.degree = 1;
            }
        }
    }

    pub(crate) fn fill_in_unknown_barycenters(&mut self, nodes: &[LNodeRef], pre_ordered: bool, random: &mut Random) {
        if pre_ordered {
            let mut last_value = -1.0;
            for index in 0..nodes.len() {
                let node = &nodes[index];
                let li = self.layer_index_of(node);
                let ni = self.node_id_of(node);
                let mut value = self.bary_state(li, ni).and_then(|s| s.barycenter);

                if value.is_none() {
                    let mut next_value = last_value + 1.0;
                    for next_node in nodes.iter().skip(index + 1) {
                        let nli = self.layer_index_of(next_node);
                        let nni = self.node_id_of(next_node);
                        if let Some(next_bary) = self.bary_state(nli, nni).and_then(|s| s.barycenter) {
                            next_value = next_bary;
                            break;
                        }
                    }
                    let computed = (last_value + next_value) / 2.0;
                    value = Some(computed);
                    if let Some(state) = self.bary_state_mut(li, ni) {
                        state.barycenter = Some(computed);
                        state.summed_weight = computed;
                        state.degree = 1;
                    }
                }

                if let Some(value) = value {
                    last_value = value;
                }
            }
        } else {
            let mut max_bary = 0.0;
            for node in nodes {
                let li = self.layer_index_of(node);
                let ni = self.node_id_of(node);
                if let Some(bary) = self.bary_state(li, ni).and_then(|s| s.barycenter) {
                    if bary > max_bary {
                        max_bary = bary;
                    }
                }
            }

            max_bary += 2.0;
            for node in nodes {
                let li = self.layer_index_of(node);
                let ni = self.node_id_of(node);
                let bary = self.bary_state(li, ni).and_then(|s| s.barycenter);
                if bary.is_none() {
                    let node_label = node
                        .lock()
                        .ok()
                        .map(|mut g| format!("id:{}", g.shape().graph_element().id))
                        .unwrap_or_else(|| "<locked>".into());
                    let raw_f = random.next_float();
                    let raw_f = random_trace::trace_next_float(
                        raw_f,
                        &format!("barycenter::fill_in_unknown_barycenters node={node_label}"),
                    );
                    let value = raw_f * max_bary - 1.0;
                    if let Some(state) = self.bary_state_mut(li, ni) {
                        state.barycenter = Some(value);
                        state.summed_weight = value;
                        state.degree = 1;
                    }
                }
            }
        }
    }

    pub(crate) fn calculate_barycenters(&mut self, nodes: &[LNodeRef], forward: bool, random: &mut Random) {
        // Clone Arc once — allows simultaneous &snapshot and &mut self access
        let snap = self.snapshot.clone();

        if let Some(ref snap) = snap {
            // Snapshot-based path: build flat→node lookup for same-layer recursion
            let mut flat_to_node: HashMap<u32, LNodeRef> = HashMap::with_capacity(nodes.len());
            for node in nodes {
                let flat = snap.node_flat_index(node);
                let li = snap.node_layer_of(flat) as usize;
                let ni = snap.node_graph_id_of(flat) as usize;
                flat_to_node.insert(flat, node.clone());
                if let Some(state) = self.bary_state_mut(li, ni) {
                    state.visited = false;
                }
            }

            let port_ranks = self.port_ranks.clone();
            for node in nodes {
                let flat = snap.node_flat_index(node);
                self.calculate_barycenter_snap(snap, flat, forward, &port_ranks, random, &flat_to_node);
            }
        } else {
            // Fallback: lock-based path (should not be reached after Step 1)
            for node in nodes {
                let li = self.layer_index_of(node);
                let ni = self.node_id_of(node);
                if let Some(state) = self.bary_state_mut(li, ni) {
                    state.visited = false;
                }
            }

            let port_ranks = self.port_ranks.clone();
            for node in nodes {
                self.calculate_barycenter_lock(node, forward, &port_ranks, random);
            }
        }
    }

    /// Snapshot-based barycenter calculation — zero locks in port/edge traversal.
    /// Uses CSR adjacency from the snapshot for cache-friendly, lock-free iteration.
    fn calculate_barycenter_snap(
        &mut self,
        snap: &CrossMinSnapshot,
        flat: u32,
        forward: bool,
        port_ranks: &[f64],
        random: &mut Random,
        flat_to_node: &HashMap<u32, LNodeRef>,
    ) {
        let li = snap.node_layer_of(flat) as usize;
        let ni = snap.node_graph_id_of(flat) as usize;

        // Check visited and reset state
        if let Some(state) = self.bary_state_mut(li, ni) {
            if state.visited {
                return;
            }
            state.visited = true;
            state.degree = 0;
            state.summed_weight = 0.0;
            state.barycenter = None;
        }

        let trace_cm = *TRACE_CROSSMIN;
        let node_name = if trace_cm {
            flat_to_node.get(&flat)
                .and_then(|n| n.lock().ok().map(|mut g| format!("id:{}", g.shape().graph_element().id)))
                .unwrap_or_else(|| "<unknown>".into())
        } else {
            String::new()
        };

        // CSR traversal — no locks for port/edge iteration
        for &pid in snap.node_ports(flat) {
            let connected = if forward {
                snap.port_predecessors(pid)
            } else {
                snap.port_successors(pid)
            };

            for &fixed_pid in connected {
                let fixed_flat = snap.port_owner_flat(fixed_pid);
                let fixed_li = snap.node_layer_of(fixed_flat) as usize;

                if fixed_li == li {
                    // Same layer — recursive barycenter computation
                    if fixed_flat != flat {
                        self.calculate_barycenter_snap(snap, fixed_flat, forward, port_ranks, random, flat_to_node);
                        let fixed_ni = snap.node_graph_id_of(fixed_flat) as usize;
                        let (degree, weight) = self.bary_state(fixed_li, fixed_ni)
                            .map(|s| (s.degree, s.summed_weight))
                            .unwrap_or((0, 0.0));
                        if let Some(state) = self.bary_state_mut(li, ni) {
                            state.degree += degree;
                            state.summed_weight += weight;
                        }
                    }
                } else {
                    // Cross-layer — use port rank directly
                    let rank = port_ranks.get(fixed_pid as usize).copied().unwrap_or(0.0);
                    if trace_cm {
                        eprintln!(
                            "[CROSSMIN] calc_bary: node={} fixed_port=pid:{} port_id={} rank={}",
                            node_name, fixed_pid, fixed_pid, rank
                        );
                    }
                    if let Some(state) = self.bary_state_mut(li, ni) {
                        state.summed_weight += rank;
                        state.degree += 1;
                    }
                }
            }
        }

        // Handle BARYCENTER_ASSOCIATES (requires one lock per node, not per port)
        let associates = flat_to_node.get(&flat).and_then(|node| {
            node.lock().ok().and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::BARYCENTER_ASSOCIATES)
            })
        });
        if let Some(associates) = associates {
            for associate in associates {
                let assoc_flat = snap.node_flat_index(&associate);
                let assoc_li = snap.node_layer_of(assoc_flat) as usize;
                if assoc_li == li {
                    self.calculate_barycenter_snap(snap, assoc_flat, forward, port_ranks, random, flat_to_node);
                    let assoc_ni = snap.node_graph_id_of(assoc_flat) as usize;
                    let (degree, weight) = self.bary_state(assoc_li, assoc_ni)
                        .map(|s| (s.degree, s.summed_weight))
                        .unwrap_or((0, 0.0));
                    if let Some(state) = self.bary_state_mut(li, ni) {
                        state.degree += degree;
                        state.summed_weight += weight;
                    }
                }
            }
        }

        // Final barycenter computation
        if let Some(state) = self.bary_state_mut(li, ni) {
            if state.degree > 0 {
                let raw_f = random.next_float();
                let raw_f = random_trace::trace_next_float(
                    raw_f,
                    &format!("barycenter::calculate_barycenter node={node_name}"),
                );
                let rand_f32 = raw_f as f32;
                let random_amount = RANDOM_AMOUNT as f32;
                let perturbation =
                    (rand_f32 * random_amount - random_amount / 2.0_f32) as f64;
                state.summed_weight += perturbation;
                state.barycenter =
                    Some(state.summed_weight / state.degree as f64);
            }
            if trace_cm {
                eprintln!(
                    "[CROSSMIN] calc_bary: node={} FINAL summed_weight={} degree={} barycenter={:?}",
                    node_name, state.summed_weight, state.degree, state.barycenter
                );
            }
        }
    }

    /// Lock-based barycenter calculation — fallback when snapshot is not available.
    fn calculate_barycenter_lock(&mut self, node: &LNodeRef, forward: bool, port_ranks: &[f64], random: &mut Random) {
        let trace_cm = *TRACE_CROSSMIN;
        let li = self.layer_index_of(node);
        let ni = self.node_id_of(node);

        if let Some(state) = self.bary_state_mut(li, ni) {
            if state.visited {
                return;
            }
            state.visited = true;
            state.degree = 0;
            state.summed_weight = 0.0;
            state.barycenter = None;
        }

        let node_name = if trace_cm {
            node.lock()
                .ok()
                .map(|mut g| format!("id:{}", g.shape().graph_element().id))
                .unwrap_or_else(|| "<locked>".into())
        } else {
            String::new()
        };

        let ports = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.ports().clone())
            .unwrap_or_default();

        for free_port in ports {
            let port_iter = if forward {
                free_port.lock().ok().map(|port_guard| port_guard.predecessor_ports()).unwrap_or_default()
            } else {
                free_port.lock().ok().map(|port_guard| port_guard.successor_ports()).unwrap_or_default()
            };

            for fixed_port in port_iter {
                let fixed_node = fixed_port.lock().ok().and_then(|port_guard| port_guard.node());
                let Some(fixed_node) = fixed_node else { continue; };

                if self.same_layer_check(&fixed_node, node) {
                    if !Arc::ptr_eq(&fixed_node, node) {
                        self.calculate_barycenter_lock(&fixed_node, forward, port_ranks, random);
                        let fli = self.layer_index_of(&fixed_node);
                        let fni = self.node_id_of(&fixed_node);
                        let (degree, weight) = self.bary_state(fli, fni)
                            .map(|s| (s.degree, s.summed_weight))
                            .unwrap_or((0, 0.0));
                        if let Some(state) = self.bary_state_mut(li, ni) {
                            state.degree += degree;
                            state.summed_weight += weight;
                        }
                    }
                } else {
                    let pid = self.port_id_of(&fixed_port);
                    let rank = port_ranks.get(pid).copied().unwrap_or(0.0);
                    if trace_cm {
                        let fp_name = fixed_port.lock().ok()
                            .map(|mut g| format!("pid:{}", g.shape().graph_element().id))
                            .unwrap_or_else(|| "<locked>".into());
                        eprintln!(
                            "[CROSSMIN] calc_bary: node={} fixed_port={} port_id={} rank={}",
                            node_name, fp_name, pid, rank
                        );
                    }
                    if let Some(state) = self.bary_state_mut(li, ni) {
                        state.summed_weight += rank;
                        state.degree += 1;
                    }
                }
            }
        }

        let associates = node.lock().ok().and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::BARYCENTER_ASSOCIATES)
        });
        if let Some(associates) = associates {
            for associate in associates {
                if self.same_layer_check(&associate, node) {
                    self.calculate_barycenter_lock(&associate, forward, port_ranks, random);
                    let ali = self.layer_index_of(&associate);
                    let ani = self.node_id_of(&associate);
                    let (degree, weight) = self.bary_state(ali, ani)
                        .map(|s| (s.degree, s.summed_weight))
                        .unwrap_or((0, 0.0));
                    if let Some(state) = self.bary_state_mut(li, ni) {
                        state.degree += degree;
                        state.summed_weight += weight;
                    }
                }
            }
        }

        if let Some(state) = self.bary_state_mut(li, ni) {
            if state.degree > 0 {
                let raw_f = random.next_float();
                let raw_f = random_trace::trace_next_float(
                    raw_f,
                    &format!("barycenter::calculate_barycenter node={node_name}"),
                );
                let rand_f32 = raw_f as f32;
                let random_amount = RANDOM_AMOUNT as f32;
                let perturbation =
                    (rand_f32 * random_amount - random_amount / 2.0_f32) as f64;
                state.summed_weight += perturbation;
                state.barycenter =
                    Some(state.summed_weight / state.degree as f64);
            }
            if trace_cm {
                eprintln!(
                    "[CROSSMIN] calc_bary: node={} FINAL summed_weight={} degree={} barycenter={:?}",
                    node_name, state.summed_weight, state.degree, state.barycenter
                );
            }
        }
    }

    fn compare_barycenter(
        &self,
        left: &LNodeRef,
        right: &LNodeRef,
        barycenter_snapshot: &HashMap<usize, Option<f64>>,
    ) -> Ordering {
        let left_bary = barycenter_snapshot
            .get(&node_ptr_id(left))
            .copied()
            .flatten();
        let right_bary = barycenter_snapshot
            .get(&node_ptr_id(right))
            .copied()
            .flatten();

        match (left_bary, right_bary) {
            (Some(left_bary), Some(right_bary)) => {
                let ord = left_bary.partial_cmp(&right_bary).unwrap_or_else(|| {
                    if *TRACE_BARYCENTER_NAN {
                        let left_name = left
                            .lock()
                            .ok()
                            .map(|mut node_guard| node_guard.to_string())
                            .unwrap_or_else(|| "<poisoned-node>".to_owned());
                        let right_name = right
                            .lock()
                            .ok()
                            .map(|mut node_guard| node_guard.to_string())
                            .unwrap_or_else(|| "<poisoned-node>".to_owned());
                        eprintln!(
                            "crossmin: barycenter nan compare left={}({}) right={}({})",
                            left_name, left_bary, right_name, right_bary
                        );
                    }
                    Ordering::Equal
                });
                if ord != Ordering::Equal {
                    return ord;
                }
                Ordering::Equal
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }

    fn barycenter_snapshot(&self, nodes: &[LNodeRef]) -> HashMap<usize, Option<f64>> {
        let mut snapshot = HashMap::with_capacity(nodes.len());
        for node in nodes {
            let barycenter = self.get_barycenter(node);
            snapshot.insert(node_ptr_id(node), barycenter);
        }
        snapshot
    }

    fn minimize_crossings_layer(
        &mut self,
        layer: &mut Vec<LNodeRef>,
        pre_ordered: bool,
        randomize: bool,
        forward: bool,
        random: &mut Random,
    ) {
        let trace = *TRACE_CROSSMIN_TIMING;
        let trace_pr = *TRACE_PORT_RANKS && self.sweep_iteration == 0;
        let start = if trace { Some(Instant::now()) } else { None };
        if randomize {
            self.randomize_barycenters(layer, random);
        } else {
            self.calculate_barycenters(layer, forward, random);
            self.fill_in_unknown_barycenters(layer, pre_ordered, random);
        }
        if let Some(start) = start {
            eprintln!(
                "crossmin: barycenter barycenters done in {} ms (randomize={})",
                start.elapsed().as_millis(),
                randomize
            );
        }

        if trace_pr && !layer.is_empty() {
            // Trace barycenters after computation
            let layer_idx = self.layer_index_of(layer.first().unwrap());
            for node in layer.iter() {
                let node_id = node
                    .lock()
                    .ok()
                    .map(|mut g| g.shape().graph_element().id)
                    .unwrap_or(-1);
                let tli = self.layer_index_of(node);
                let tni = self.node_id_of(node);
                let (barycenter, summed_weight, degree) = self
                    .bary_state(tli, tni)
                    .map(|s| (s.barycenter, s.summed_weight, s.degree))
                    .unwrap_or((None, 0.0, 0));
                let bary_val = barycenter.unwrap_or(f64::NAN);
                eprintln!(
                    "[BARYCENTER]\t0\t{}\t{}\t{}\t{}\t{}",
                    layer_idx, node_id, bary_val, summed_weight, degree
                );
            }
            // Trace node order before sort
            let node_ids_before: Vec<i32> = layer
                .iter()
                .map(|n| {
                    n.lock()
                        .ok()
                        .map(|mut g| g.shape().graph_element().id)
                        .unwrap_or(-1)
                })
                .collect();
            let ids_str_before: Vec<String> =
                node_ids_before.iter().map(|id| id.to_string()).collect();
            eprintln!(
                "[NODE_ORDER]\t0\t{}\tbefore\t{}",
                layer_idx,
                ids_str_before.join("\t")
            );
        }

        let trace_layer_pattern = TRACE_BARYCENTER_LAYER_PATTERN.clone();
        if trace_layer_pattern.as_ref().is_some_and(|pattern| {
            layer.iter().any(|node| {
                node.lock()
                    .ok()
                    .map(|mut node_guard| node_guard.to_string().contains(pattern))
                    .unwrap_or(false)
            })
        }) {
            eprintln!(
                "crossmin: barycenter layer_state pre_ordered={} randomize={} forward={}",
                pre_ordered, randomize, forward
            );
            for (index, node) in layer.iter().enumerate() {
                let name = node
                    .lock()
                    .ok()
                    .map(|mut node_guard| node_guard.to_string())
                    .unwrap_or_else(|| "<poisoned-node>".to_owned());
                let tli = self.layer_index_of(node);
                let tni = self.node_id_of(node);
                let (barycenter, degree, summed_weight) = self
                    .bary_state(tli, tni)
                    .map(|s| (s.barycenter, s.degree, s.summed_weight))
                    .unwrap_or((None, 0, 0.0));
                eprintln!(
                    "crossmin: barycenter node[{}]={} bary={:?} degree={} sum={}",
                    index, name, barycenter, degree, summed_weight
                );
            }
        }

        if layer.len() > 1 {
            let sort_start = if trace { Some(Instant::now()) } else { None };
            let barycenter_snapshot = self.barycenter_snapshot(layer);
            let mut entries: Vec<(usize, LNodeRef)> = layer.iter().cloned().enumerate().collect();
            entries.sort_by(|(left_index, left), (right_index, right)| {
                let ord = self.compare_barycenter(left, right, &barycenter_snapshot);
                if ord == Ordering::Equal {
                    left_index.cmp(right_index)
                } else {
                    ord
                }
            });
            layer.clear();
            layer.extend(entries.into_iter().map(|(_, node)| node));
            self.constraint_resolver.process_constraints(layer);
            if let Some(sort_start) = sort_start {
                eprintln!(
                    "crossmin: barycenter sort+constraints done in {} ms (len={})",
                    sort_start.elapsed().as_millis(),
                    layer.len()
                );
            }
        }

        if trace_pr && !layer.is_empty() {
            let layer_idx = self.layer_index_of(layer.first().unwrap());
            let node_ids_after: Vec<i32> = layer
                .iter()
                .map(|n| {
                    n.lock()
                        .ok()
                        .map(|mut g| g.shape().graph_element().id)
                        .unwrap_or(-1)
                })
                .collect();
            let ids_str_after: Vec<String> =
                node_ids_after.iter().map(|id| id.to_string()).collect();
            eprintln!(
                "[NODE_ORDER]\t0\t{}\tafter\t{}",
                layer_idx,
                ids_str_after.join("\t")
            );
        }

        if *TRACE_CROSSMIN {
            let li = layer
                .first()
                .and_then(|n| n.lock().ok().and_then(|ng| ng.layer()))
                .and_then(|l| l.lock().ok().map(|mut lg| lg.graph_element().id))
                .unwrap_or(-1);
            eprintln!(
                "[CROSSMIN] minimize_crossings_layer: layer={} forward={} nodes={}",
                li,
                forward,
                layer.len()
            );
            for (i, node) in layer.iter().enumerate() {
                let tli = self.layer_index_of(node);
                let tni = self.node_id_of(node);
                let (name, bary, deg, sw) = self
                    .bary_state(tli, tni)
                    .map(|sg| {
                        let nm = node
                            .lock()
                            .ok()
                            .map(|mut g| {
                                format!("id:{}", g.shape().graph_element().id)
                            })
                            .unwrap_or_else(|| "<locked>".into());
                        (nm, sg.barycenter, sg.degree, sg.summed_weight)
                    })
                    .unwrap_or(("<no_state>".into(), None, 0, 0.0));
                eprintln!(
                    "[CROSSMIN] minimize_crossings_layer: layer={} node[{}]={} barycenter={:?} degree={} summed_weight={}",
                    li, i, name, bary, deg, sw
                );
            }
        }
    }

    pub(crate) fn is_external_port_dummy(&self, node: &LNodeRef) -> bool {
        node.lock()
            .ok()
            .map(|node_guard| node_guard.node_type() == NodeType::ExternalPort)
            .unwrap_or(false)
    }

    pub(crate) fn is_first_layer(
        &self,
        node_order: &[Vec<LNodeRef>],
        current_index: usize,
        forward_sweep: bool,
    ) -> bool {
        let start_index = if forward_sweep {
            0
        } else {
            node_order.len().saturating_sub(1)
        };
        current_index == start_index
    }
}

impl ICrossingMinimizationHeuristic for BarycenterHeuristic {
    fn always_improves(&self) -> bool {
        false
    }

    fn set_first_layer_order(&mut self, order: &mut [Vec<LNodeRef>], forward_sweep: bool, random: &mut Random) -> bool {
        let start_index = if forward_sweep {
            0
        } else {
            order.len().saturating_sub(1)
        };
        let mut nodes = order[start_index].clone();
        self.minimize_crossings_layer(&mut nodes, false, true, forward_sweep, random);
        order[start_index] = nodes;
        false
    }

    fn minimize_crossings(
        &mut self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
        is_first_sweep: bool,
        random: &mut Random,
    ) -> bool {
        if !self.is_first_layer(order, free_layer_index, forward_sweep) {
            let fixed_layer_index = if forward_sweep {
                free_layer_index.saturating_sub(1)
            } else {
                free_layer_index + 1
            };
            let port_type = if forward_sweep {
                PortType::Output
            } else {
                PortType::Input
            };
            if let Some(layer) = order.get(fixed_layer_index) {
                self.port_distributor.calculate_port_ranks(layer, port_type);
                self.port_ranks = self.port_distributor.port_ranks();
            }
        }

        let pre_ordered = !is_first_sweep
            || order
                .get(free_layer_index)
                .and_then(|layer| layer.first())
                .map(|node| self.is_external_port_dummy(node))
                .unwrap_or(false);

        let mut nodes = order[free_layer_index].clone();
        self.minimize_crossings_layer(&mut nodes, pre_ordered, false, forward_sweep, random);
        order[free_layer_index] = nodes;
        false
    }

    fn is_deterministic(&self) -> bool {
        false
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn reset_sweep_iteration(&mut self) {
        self.sweep_iteration = 0;
    }

    fn increment_sweep_iteration(&mut self) {
        self.sweep_iteration += 1;
    }
}

impl IInitializable for BarycenterHeuristic {
    fn init_after_traversal(&mut self) {
        // Java initializes the resolver/distributor separately in GraphInfoHolder and
        // only snapshots the computed arrays here.
        // Barycenter states are now accessed directly through self.constraint_resolver.barycenter_states.
        self.port_ranks = self.port_distributor.port_ranks();
    }

    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        self.constraint_resolver
            .init_at_layer_level(layer_index, node_order);
        if let Some(first_node) = node_order[layer_index].first() {
            if let Some(layer) = first_node
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.layer())
            {
                if let Ok(mut layer_guard) = layer.lock() {
                    layer_guard.graph_element().id = layer_index as i32;
                }
            }
        }
    }

    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.constraint_resolver
            .init_at_node_level(layer_index, node_index, node_order);
    }

    fn init_at_port_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.constraint_resolver.init_at_port_level(
            layer_index,
            node_index,
            port_index,
            node_order,
        );
    }

    fn init_at_edge_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        port_index: usize,
        edge_index: usize,
        edge: &crate::org::eclipse::elk::alg::layered::graph::LEdgeRef,
        node_order: &[Vec<LNodeRef>],
    ) {
        self.constraint_resolver.init_at_edge_level(
            layer_index,
            node_index,
            port_index,
            edge_index,
            edge,
            node_order,
        );
    }
}

#[derive(Clone)]
pub struct BarycenterState {
    pub node: LNodeRef,
    pub summed_weight: f64,
    pub degree: i32,
    pub barycenter: Option<f64>,
    pub visited: bool,
}

impl BarycenterState {
    pub fn new(node: LNodeRef) -> Self {
        BarycenterState {
            node,
            summed_weight: 0.0,
            degree: 0,
            barycenter: None,
            visited: false,
        }
    }
}

// Java uses 0.07f (f32 literal). Match f32 precision for parity.
const RANDOM_AMOUNT: f64 = 0.07_f32 as f64;

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn layer_index(node: &LNodeRef) -> usize {
    let layer = node.lock().ok().and_then(|node_guard| node_guard.layer());
    if let Some(layer) = layer {
        if let Ok(mut layer_guard) = layer.lock() {
            return layer_guard.graph_element().id as usize;
        }
    }
    0
}

fn node_ptr_id(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}

fn same_layer(left: &LNodeRef, right: &LNodeRef) -> bool {
    let left_layer = left.lock().ok().and_then(|node_guard| node_guard.layer());
    let right_layer = right.lock().ok().and_then(|node_guard| node_guard.layer());
    match (left_layer, right_layer) {
        (Some(left_layer), Some(right_layer)) => Arc::ptr_eq(&left_layer, &right_layer),
        _ => false,
    }
}

fn port_id(port: &LPortRef) -> usize {
    port.lock()
        .ok()
        .map(|mut port_guard| port_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}
