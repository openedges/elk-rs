use std::any::Any;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use org_eclipse_elk_core::org::eclipse::elk::core::util::Random;
use crate::org::eclipse::elk::alg::layered::p3order::random_trace;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::{InternalProperties, PortType};
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_port_distributor::BarycenterPortDistributor;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::forster_constraint_resolver::ForsterConstraintResolver;
use crate::org::eclipse::elk::alg::layered::p3order::i_crossing_minimization_heuristic::ICrossingMinimizationHeuristic;

pub struct BarycenterHeuristic {
    pub(crate) port_ranks: Vec<f64>,
    random: Random,
    pub(crate) constraint_resolver: ForsterConstraintResolver,
    barycenter_state: Vec<Vec<Arc<Mutex<BarycenterState>>>>,
    pub(crate) port_distributor: Box<dyn BarycenterPortDistributor>,
}

impl BarycenterHeuristic {
    pub fn new(
        constraint_resolver: ForsterConstraintResolver,
        random: Random,
        port_distributor: Box<dyn BarycenterPortDistributor>,
    ) -> Self {
        BarycenterHeuristic {
            port_ranks: Vec::new(),
            random,
            constraint_resolver,
            barycenter_state: Vec::new(),
            port_distributor,
        }
    }

    pub fn set_random(&mut self, random: Random) {
        self.random = random;
    }

    pub fn random(&self) -> Random {
        self.random.clone()
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random.set_seed(seed);
    }

    pub(crate) fn randomize_barycenters(&mut self, nodes: &[LNodeRef]) {
        for node in nodes {
            let node_label = node
                .lock()
                .ok()
                .map(|mut g| format!("id:{}", g.shape().graph_element().id))
                .unwrap_or_else(|| "<locked>".into());
            let raw = self.random.next_double();
            let value = random_trace::trace_next_double(
                raw,
                &format!("barycenter::randomize_barycenters node={node_label}"),
            );
            if let Some(state) = self.state_of(node) {
                if let Ok(mut state_guard) = state.lock() {
                    state_guard.barycenter = Some(value);
                    state_guard.summed_weight = value;
                    state_guard.degree = 1;
                }
            }
        }
    }

    pub(crate) fn fill_in_unknown_barycenters(&mut self, nodes: &[LNodeRef], pre_ordered: bool) {
        if pre_ordered {
            let mut last_value = -1.0;
            for index in 0..nodes.len() {
                let node = &nodes[index];
                let mut value = self.state_of(node).and_then(|state| {
                    state
                        .lock()
                        .ok()
                        .and_then(|state_guard| state_guard.barycenter)
                });

                if value.is_none() {
                    let mut next_value = last_value + 1.0;
                    for next_node in nodes.iter().skip(index + 1) {
                        if let Some(next_bary) = self.state_of(next_node).and_then(|state| {
                            state
                                .lock()
                                .ok()
                                .and_then(|state_guard| state_guard.barycenter)
                        }) {
                            next_value = next_bary;
                            break;
                        }
                    }
                    let computed = (last_value + next_value) / 2.0;
                    value = Some(computed);
                    if let Some(state) = self.state_of(node) {
                        if let Ok(mut state_guard) = state.lock() {
                            state_guard.barycenter = Some(computed);
                            state_guard.summed_weight = computed;
                            state_guard.degree = 1;
                        }
                    }
                }

                if let Some(value) = value {
                    last_value = value;
                }
            }
        } else {
            let mut max_bary = 0.0;
            for node in nodes {
                if let Some(bary) = self.state_of(node).and_then(|state| {
                    state
                        .lock()
                        .ok()
                        .and_then(|state_guard| state_guard.barycenter)
                }) {
                    if bary > max_bary {
                        max_bary = bary;
                    }
                }
            }

            max_bary += 2.0;
            for node in nodes {
                let bary = self.state_of(node).and_then(|state| {
                    state
                        .lock()
                        .ok()
                        .and_then(|state_guard| state_guard.barycenter)
                });
                if bary.is_none() {
                    let node_label = node
                        .lock()
                        .ok()
                        .map(|mut g| format!("id:{}", g.shape().graph_element().id))
                        .unwrap_or_else(|| "<locked>".into());
                    let raw_f = self.random.next_float();
                    let raw_f = random_trace::trace_next_float(
                        raw_f,
                        &format!("barycenter::fill_in_unknown_barycenters node={node_label}"),
                    );
                    let value = raw_f * max_bary - 1.0;
                    if let Some(state) = self.state_of(node) {
                        if let Ok(mut state_guard) = state.lock() {
                            state_guard.barycenter = Some(value);
                            state_guard.summed_weight = value;
                            state_guard.degree = 1;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn calculate_barycenters(&mut self, nodes: &[LNodeRef], forward: bool) {
        for node in nodes {
            if let Some(state) = self.state_of(node) {
                if let Ok(mut state_guard) = state.lock() {
                    state_guard.visited = false;
                }
            }
        }

        let port_ranks = self.port_ranks.clone();
        for node in nodes {
            self.calculate_barycenter(node, forward, &port_ranks);
        }
    }

    fn calculate_barycenter(&mut self, node: &LNodeRef, forward: bool, port_ranks: &[f64]) {
        let trace_cm = std::env::var_os("ELK_TRACE_CROSSMIN").is_some();
        if let Some(state) = self.state_of(node) {
            if let Ok(mut state_guard) = state.lock() {
                if state_guard.visited {
                    return;
                }
                state_guard.visited = true;
                state_guard.degree = 0;
                state_guard.summed_weight = 0.0;
                state_guard.barycenter = None;
            }
        }

        let node_name = if trace_cm {
            node.lock()
                .ok()
                .map(|mut g| {
                    format!("id:{}", g.shape().graph_element().id)
                })
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
                free_port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.predecessor_ports())
                    .unwrap_or_default()
            } else {
                free_port
                    .lock()
                    .ok()
                    .map(|port_guard| port_guard.successor_ports())
                    .unwrap_or_default()
            };

            for fixed_port in port_iter {
                let fixed_node = fixed_port
                    .lock()
                    .ok()
                    .and_then(|port_guard| port_guard.node());
                let Some(fixed_node) = fixed_node else {
                    continue;
                };

                if same_layer(&fixed_node, node) {
                    if !Arc::ptr_eq(&fixed_node, node) {
                        self.calculate_barycenter(&fixed_node, forward, port_ranks);
                        let (degree, weight) = self.state_values(&fixed_node);
                        if let Some(state) = self.state_of(node) {
                            if let Ok(mut state_guard) = state.lock() {
                                state_guard.degree += degree;
                                state_guard.summed_weight += weight;
                            }
                        }
                    }
                } else {
                    let pid = port_id(&fixed_port);
                    let rank = port_ranks.get(pid).copied().unwrap_or(0.0);
                    if trace_cm {
                        let fp_name = fixed_port
                            .lock()
                            .ok()
                            .map(|mut g| {
                                format!("pid:{}", g.shape().graph_element().id)
                            })
                            .unwrap_or_else(|| "<locked>".into());
                        eprintln!(
                            "[CROSSMIN] calc_bary: node={} fixed_port={} port_id={} rank={}",
                            node_name, fp_name, pid, rank
                        );
                    }
                    if let Some(state) = self.state_of(node) {
                        if let Ok(mut state_guard) = state.lock() {
                            state_guard.summed_weight += rank;
                            state_guard.degree += 1;
                        }
                    }
                }
            }
        }

        let associates = node.lock().ok().and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::BARYCENTER_ASSOCIATES)
        });
        if let Some(associates) = associates {
            for associate in associates {
                if same_layer(&associate, node) {
                    self.calculate_barycenter(&associate, forward, port_ranks);
                    let (degree, weight) = self.state_values(&associate);
                    if let Some(state) = self.state_of(node) {
                        if let Ok(mut state_guard) = state.lock() {
                            state_guard.degree += degree;
                            state_guard.summed_weight += weight;
                        }
                    }
                }
            }
        }

        if let Some(state) = self.state_of(node) {
            if let Ok(mut state_guard) = state.lock() {
                if state_guard.degree > 0 {
                    // Java: float perturbation = random.nextFloat() * RANDOM_AMOUNT - RANDOM_AMOUNT / 2.0f
                    // All float (f32) arithmetic, then promoted to double for +=
                    let raw_f = self.random.next_float();
                    let raw_f = random_trace::trace_next_float(
                        raw_f,
                        &format!("barycenter::calculate_barycenter node={node_name}"),
                    );
                    let rand_f32 = raw_f as f32;
                    let random_amount = RANDOM_AMOUNT as f32;
                    let perturbation =
                        (rand_f32 * random_amount - random_amount / 2.0_f32) as f64;
                    state_guard.summed_weight += perturbation;
                    state_guard.barycenter =
                        Some(state_guard.summed_weight / state_guard.degree as f64);
                }
                if trace_cm {
                    eprintln!(
                        "[CROSSMIN] calc_bary: node={} FINAL summed_weight={} degree={} barycenter={:?}",
                        node_name, state_guard.summed_weight, state_guard.degree, state_guard.barycenter
                    );
                }
            }
        }
    }

    pub(crate) fn state_of(&self, node: &LNodeRef) -> Option<Arc<Mutex<BarycenterState>>> {
        let layer_index = layer_index(node);
        let node_index = node_id(node);
        self.barycenter_state
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
            .cloned()
    }

    fn state_values(&self, node: &LNodeRef) -> (i32, f64) {
        if let Some(state) = self.state_of(node) {
            if let Ok(state_guard) = state.lock() {
                return (state_guard.degree, state_guard.summed_weight);
            }
        }
        (0, 0.0)
    }

    fn compare_barycenter(&self, left: &LNodeRef, right: &LNodeRef) -> Ordering {
        let left_bary = self.state_of(left).and_then(|state| {
            state
                .lock()
                .ok()
                .and_then(|state_guard| state_guard.barycenter)
        });
        let right_bary = self.state_of(right).and_then(|state| {
            state
                .lock()
                .ok()
                .and_then(|state_guard| state_guard.barycenter)
        });

        match (left_bary, right_bary) {
            (Some(left_bary), Some(right_bary)) => {
                let ord = left_bary.partial_cmp(&right_bary).unwrap_or_else(|| {
                    if std::env::var_os("ELK_TRACE_BARYCENTER_NAN").is_some() {
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

    fn minimize_crossings_layer(
        &mut self,
        layer: &mut Vec<LNodeRef>,
        pre_ordered: bool,
        randomize: bool,
        forward: bool,
    ) {
        let trace = std::env::var_os("ELK_TRACE_CROSSMIN_TIMING").is_some();
        let start = if trace { Some(Instant::now()) } else { None };
        if randomize {
            self.randomize_barycenters(layer);
        } else {
            self.calculate_barycenters(layer, forward);
            self.fill_in_unknown_barycenters(layer, pre_ordered);
        }
        if let Some(start) = start {
            eprintln!(
                "crossmin: barycenter barycenters done in {} ms (randomize={})",
                start.elapsed().as_millis(),
                randomize
            );
        }

        if std::env::var_os("ELK_TRACE_BARYCENTER_LAYER").is_some()
            && layer.iter().any(|node| {
                node.lock()
                    .ok()
                    .map(|mut node_guard| node_guard.to_string().contains("pumpOutletPressure"))
                    .unwrap_or(false)
            })
        {
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
                let (barycenter, degree, summed_weight) = self
                    .state_of(node)
                    .and_then(|state| {
                        state.lock().ok().map(|state_guard| {
                            (
                                state_guard.barycenter,
                                state_guard.degree,
                                state_guard.summed_weight,
                            )
                        })
                    })
                    .unwrap_or((None, 0, 0.0));
                eprintln!(
                    "crossmin: barycenter node[{}]={} bary={:?} degree={} sum={}",
                    index, name, barycenter, degree, summed_weight
                );
            }
        }

        if layer.len() > 1 {
            let sort_start = if trace { Some(Instant::now()) } else { None };
            let mut entries: Vec<(usize, LNodeRef)> = layer.iter().cloned().enumerate().collect();
            entries.sort_by(|(left_index, left), (right_index, right)| {
                let ord = self.compare_barycenter(left, right);
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

        if std::env::var_os("ELK_TRACE_CROSSMIN").is_some() {
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
                let (name, bary, deg, sw) = self
                    .state_of(node)
                    .and_then(|st| {
                        st.lock().ok().map(|sg| {
                            let nm = node
                                .lock()
                                .ok()
                                .map(|mut g| {
                                    format!("id:{}", g.shape().graph_element().id)
                                })
                                .unwrap_or_else(|| "<locked>".into());
                            (nm, sg.barycenter, sg.degree, sg.summed_weight)
                        })
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

    fn set_first_layer_order(&mut self, order: &mut [Vec<LNodeRef>], forward_sweep: bool) -> bool {
        let start_index = if forward_sweep {
            0
        } else {
            order.len().saturating_sub(1)
        };
        let mut nodes = order[start_index].clone();
        self.minimize_crossings_layer(&mut nodes, false, true, forward_sweep);
        order[start_index] = nodes;
        false
    }

    fn minimize_crossings(
        &mut self,
        order: &mut [Vec<LNodeRef>],
        free_layer_index: usize,
        forward_sweep: bool,
        is_first_sweep: bool,
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
        self.minimize_crossings_layer(&mut nodes, pre_ordered, false, forward_sweep);
        order[free_layer_index] = nodes;
        false
    }

    fn is_deterministic(&self) -> bool {
        false
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl IInitializable for BarycenterHeuristic {
    fn init_after_traversal(&mut self) {
        // Java initializes the resolver/distributor separately in GraphInfoHolder and
        // only snapshots the computed arrays here.
        self.barycenter_state = self.constraint_resolver.barycenter_states();
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
