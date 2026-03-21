use std::collections::VecDeque;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

static DEBUG_CYCLE_RANDOM_PREFETCH: LazyLock<Option<usize>> = LazyLock::new(|| {
    ElkTrace::global()
        .debug_cycle_random_prefetch
        .as_ref()
        .and_then(|value| value.parse::<usize>().ok())
});
static DEBUG_CYCLE_FORCE_REVERSE_ORIGINS: LazyLock<Option<HashSet<usize>>> = LazyLock::new(|| {
    let raw = ElkTrace::global().debug_cycle_force_reverse_origins.as_ref()?;
    let values = raw
        .split(',')
        .filter_map(|token| token.trim().parse::<usize>().ok())
        .collect::<HashSet<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
});

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Random};

use crate::org::eclipse::elk::alg::layered::graph::{LEdgeRef, LGraph, LGraphUtil, LNodeRef, LPortRef};
use crate::org::eclipse::elk::alg::layered::options::internal_properties::Origin;
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions,
};
use crate::org::eclipse::elk::alg::layered::p1cycles::group_model_order_calculator::GroupModelOrderCalculator;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static INTERMEDIATE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_after(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::ReversedEdgeRestorer),
    );
    config
});

pub struct GreedyCycleBreaker {
    indeg: Vec<i32>,
    outdeg: Vec<i32>,
    mark: Vec<i32>,
    sources: VecDeque<LNodeRef>,
    sinks: VecDeque<LNodeRef>,
    random: Random,
    prefer_model_order: bool,
}

impl GreedyCycleBreaker {
    pub fn new() -> Self {
        GreedyCycleBreaker {
            indeg: Vec::new(),
            outdeg: Vec::new(),
            mark: Vec::new(),
            sources: VecDeque::new(),
            sinks: VecDeque::new(),
            random: Random::new(0),
            prefer_model_order: false,
        }
    }

    pub fn new_with_model_order(prefer_model_order: bool) -> Self {
        let mut cycle_breaker = Self::new();
        cycle_breaker.prefer_model_order = prefer_model_order;
        cycle_breaker
    }

    fn choose_node_with_max_outflow(
        &mut self,
        layered_graph: &mut LGraph,
        nodes: &[LNodeRef],
    ) -> Option<LNodeRef> {
        if nodes.is_empty() {
            return None;
        }

        if self.prefer_model_order {
            let offset = std::cmp::max(
                layered_graph.layerless_nodes().len() as i32,
                layered_graph
                    .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
                    .unwrap_or(0),
            );
            let big_offset = offset
                * layered_graph
                    .get_property(InternalProperties::CB_NUM_MODEL_ORDER_GROUPS)
                    .unwrap_or(0);
            let enforce_group_model_order = layered_graph
                .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY)
                .unwrap_or(GroupOrderStrategy::OnlyWithinGroup)
                == GroupOrderStrategy::Enforced;

            let mut minimum_model_order = i32::MAX;
            let mut return_node: Option<LNodeRef> = None;
            let mut model_order_calculator = GroupModelOrderCalculator::new();

            for node in nodes {
                let has_model_order = node.lock_ok().is_some_and(|mut node_guard| {
                    node_guard
                        .shape()
                        .graph_element()
                        .properties()
                        .has_property(InternalProperties::MODEL_ORDER)
                });
                if !has_model_order {
                    continue;
                }

                let model_order = if enforce_group_model_order {
                    model_order_calculator
                        .compute_constraint_group_model_order(node, big_offset, offset)
                } else {
                    model_order_calculator.compute_constraint_model_order(node, offset)
                };
                if minimum_model_order > model_order {
                    minimum_model_order = model_order;
                    return_node = Some(node.clone());
                }
            }

            if return_node.is_some() {
                return return_node;
            }
        }

        let trace_choices = ElkTrace::global().cycle_choices;
        let index = self.random.next_int(nodes.len() as i32) as usize;
        if trace_choices {
            let candidates = nodes
                .iter()
                .map(node_index)
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(",");
            eprintln!(
                "[cycle-breaker-choice] candidates=[{}] picked_index={} picked_node={}",
                candidates,
                index,
                nodes.get(index).map(node_index).unwrap_or_default()
            );
        }
        nodes.get(index).cloned()
    }

    fn update_neighbors(&mut self, node: &LNodeRef) {
        let ports = match node.lock_ok() {
            Some(node_guard) => node_guard.ports().clone(),
            None => return,
        };

        for port in ports {
            let edges = match port.lock_ok() {
            Some(port_guard) => port_guard.connected_edges(),
            None => Vec::new(),
            };

            for edge in edges {
                let (connected_port, is_target, priority) = match edge.lock_ok() {
            Some(mut edge_guard) => {
                        let source = edge_guard.source();
                        let target = edge_guard.target();
                        let Some(source_port) = source else {
                            continue;
                        };
                        let Some(target_port) = target else {
                            continue;
                        };
                        let connected_port = if Arc::ptr_eq(&source_port, &port) {
                            target_port.clone()
                        } else {
                            source_port
                        };
                        let is_target = Arc::ptr_eq(&target_port, &connected_port);
                        let priority = edge_guard
                            .get_property(LayeredOptions::PRIORITY_DIRECTION)
                            .unwrap_or(0);
                        (connected_port, is_target, priority)
                    }
            None => continue,
                };

                let endpoint = connected_port
                    .lock_ok()
                    .and_then(|port_guard| port_guard.node());
                let Some(endpoint) = endpoint else {
                    continue;
                };

                if Arc::ptr_eq(node, &endpoint) {
                    continue;
                }

                let index = node_index(&endpoint);
                if index >= self.mark.len() {
                    continue;
                }

                if self.mark[index] != 0 {
                    continue;
                }

                let priority = if priority < 0 { 0 } else { priority };
                if is_target {
                    self.indeg[index] -= priority + 1;
                    if self.indeg[index] <= 0 && self.outdeg[index] > 0 {
                        self.sources.push_back(endpoint);
                    }
                } else {
                    self.outdeg[index] -= priority + 1;
                    if self.outdeg[index] <= 0 && self.indeg[index] > 0 {
                        self.sinks.push_back(endpoint);
                    }
                }
            }
        }
    }

    fn reverse_edges(&mut self, graph: &mut LGraph, nodes: &[LNodeRef]) {
        let reverse_graph = nodes
            .first()
            .and_then(|node| node.lock_ok().and_then(|node_guard| node_guard.graph()))
            .unwrap_or_default();
        let trace_reversals = ElkTrace::global().cycle_reversals;
        let forced_reversed = DEBUG_CYCLE_FORCE_REVERSE_ORIGINS.as_ref();
        let mut reversed_edges = Vec::new();
        for node in nodes {
            let (ports, node_idx) = match node.lock_ok() {
            Some(mut node_guard) => {
                    let node_idx = node_guard.shape().graph_element().id as usize;
                    (node_guard.ports().clone(), node_idx)
                }
            None => continue,
            };

            for port in LGraphUtil::to_port_array(&ports) {
                let edges = match port.lock_ok() {
            Some(port_guard) => port_guard.outgoing_edges().clone(),
            None => Vec::new(),
                };

                for edge in LGraphUtil::to_edge_array(&edges) {
                    let target_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));
                    let Some(target_node) = target_node else {
                        continue;
                    };
                    let target_index = node_index(&target_node);
                    let reverse_by_mark = node_idx < self.mark.len()
                        && target_index < self.mark.len()
                        && self.mark[node_idx] > self.mark[target_index];
                    let should_reverse = if let Some(forced) = &forced_reversed {
                        edge_origin_id(&edge).is_some_and(|origin_id| forced.contains(&origin_id))
                    } else {
                        reverse_by_mark
                    };
                    if should_reverse
                    {
                        if trace_reversals {
                            reversed_edges.push(trace_reversal_entry(
                                &edge,
                                node_idx,
                                target_index,
                                self.mark[node_idx],
                                self.mark[target_index],
                            ));
                        }
                        crate::org::eclipse::elk::alg::layered::graph::LEdge::reverse(
                            &edge,
                            &reverse_graph,
                            true,
                        );
                        graph.set_property(InternalProperties::CYCLIC, Some(true));
                    }
                }
            }
        }

        if trace_reversals && !reversed_edges.is_empty() {
            reversed_edges.sort();
            eprintln!(
                "[cycle-breaker] reversed_count={} edges={}",
                reversed_edges.len(),
                reversed_edges.join(" | ")
            );
        }
    }
}

impl Default for GreedyCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for GreedyCycleBreaker {
    fn type_name(&self) -> &'static str {
        if self.prefer_model_order {
            "GreedyModelOrderCycleBreaker"
        } else {
            "GreedyCycleBreaker"
        }
    }

    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Greedy cycle removal", 1.0);

        let nodes = layered_graph.layerless_nodes().clone();
        let unprocessed_total = nodes.len();
        self.indeg = vec![0; unprocessed_total];
        self.outdeg = vec![0; unprocessed_total];
        self.mark = vec![0; unprocessed_total];
        self.sources.clear();
        self.sinks.clear();
        self.random = layered_graph
            .get_property(InternalProperties::RANDOM)
            .unwrap_or_else(|| Random::new(0));
        if let Some(prefetch) = *DEBUG_CYCLE_RANDOM_PREFETCH {
            for _ in 0..prefetch {
                let _ = self.random.next_int(2);
            }
            if ElkTrace::global().cycle_choices {
                eprintln!("[cycle-breaker-choice] random_prefetch={prefetch}");
            }
        }

        for (index, node) in nodes.iter().enumerate() {
            if let Some(mut node_guard) = node.lock_ok() {
                node_guard.shape().graph_element().id = index as i32;
            }

            let ports = match node.lock_ok() {
            Some(node_guard) => node_guard.ports().clone(),
            None => continue,
            };

            for port in ports {
                let (incoming, outgoing) = match port.lock_ok() {
            Some(port_guard) => (
                        port_guard.incoming_edges().clone(),
                        port_guard.outgoing_edges().clone(),
                    ),
            None => (Vec::new(), Vec::new()),
                };

                for edge in incoming {
                    let source_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.source())
                        .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));
                    if source_node
                        .as_ref()
                        .is_some_and(|source| Arc::ptr_eq(source, node))
                    {
                        continue;
                    }
                    let priority = edge
                        .lock_ok()
                        .and_then(|mut edge_guard| {
                            edge_guard.get_property(LayeredOptions::PRIORITY_DIRECTION)
                        })
                        .unwrap_or(0);
                    self.indeg[index] += if priority > 0 { priority + 1 } else { 1 };
                }

                for edge in outgoing {
                    let target_node = edge
                        .lock_ok()
                        .and_then(|edge_guard| edge_guard.target())
                        .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));
                    if target_node
                        .as_ref()
                        .is_some_and(|target| Arc::ptr_eq(target, node))
                    {
                        continue;
                    }
                    let priority = edge
                        .lock_ok()
                        .and_then(|mut edge_guard| {
                            edge_guard.get_property(LayeredOptions::PRIORITY_DIRECTION)
                        })
                        .unwrap_or(0);
                    self.outdeg[index] += if priority > 0 { priority + 1 } else { 1 };
                }
            }

            if self.outdeg[index] == 0 {
                self.sinks.push_back(node.clone());
            } else if self.indeg[index] == 0 {
                self.sources.push_back(node.clone());
            }
        }

        let mut unprocessed = unprocessed_total;
        let mut next_right: i32 = -1;
        let mut next_left: i32 = 1;
        let mut max_nodes: Vec<LNodeRef> = Vec::new();

        while unprocessed > 0 {
            while let Some(sink) = self.sinks.pop_front() {
                let index = node_index(&sink);
                if index < self.mark.len() {
                    self.mark[index] = next_right;
                    next_right -= 1;
                    self.update_neighbors(&sink);
                    unprocessed -= 1;
                }
            }

            while let Some(source) = self.sources.pop_front() {
                let index = node_index(&source);
                if index < self.mark.len() {
                    self.mark[index] = next_left;
                    next_left += 1;
                    self.update_neighbors(&source);
                    unprocessed -= 1;
                }
            }

            if unprocessed > 0 {
                let mut max_outflow = i32::MIN;
                max_nodes.clear();
                for node in &nodes {
                    let index = node_index(node);
                    if index < self.mark.len() && self.mark[index] == 0 {
                        let outflow = self.outdeg[index] - self.indeg[index];
                        if outflow >= max_outflow {
                            if outflow > max_outflow {
                                max_nodes.clear();
                                max_outflow = outflow;
                            }
                            max_nodes.push(node.clone());
                        }
                    }
                }

                if let Some(max_node) = self.choose_node_with_max_outflow(layered_graph, &max_nodes)
                {
                    let index = node_index(&max_node);
                    if index < self.mark.len() {
                        self.mark[index] = next_left;
                        next_left += 1;
                        self.update_neighbors(&max_node);
                        unprocessed -= 1;
                    }
                }
            }
        }

        let shift_base = nodes.len() as i32 + 1;
        for value in &mut self.mark {
            if *value < 0 {
                *value += shift_base;
            }
        }

        self.reverse_edges(layered_graph, &nodes);
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(LayoutProcessorConfiguration::create_from(
            &INTERMEDIATE_PROCESSING_CONFIGURATION,
        ))
    }
}

fn node_index(node: &LNodeRef) -> usize {
    node.lock_ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn trace_reversal_entry(
    edge: &LEdgeRef,
    source_node_idx: usize,
    target_node_idx: usize,
    source_mark: i32,
    target_mark: i32,
) -> String {
    let (edge_origin, source_port_origin, target_port_origin) = edge
        .lock_ok()
        .map(|mut edge_guard| {
            let edge_origin = edge_guard
                .get_property(InternalProperties::ORIGIN)
                .and_then(|origin| match origin {
                    Origin::ElkEdge(origin_id) => Some(origin_id),
                    _ => None,
                });
            let source_port_origin = edge_guard.source().and_then(|port| trace_port_origin(&port));
            let target_port_origin = edge_guard.target().and_then(|port| trace_port_origin(&port));
            (edge_origin, source_port_origin, target_port_origin)
        })
        .unwrap_or((None, None, None));

    format!(
        "edge_origin={:?},source_port_origin={:?},target_port_origin={:?},src_node={},tgt_node={},src_mark={},tgt_mark={}",
        edge_origin,
        source_port_origin,
        target_port_origin,
        source_node_idx,
        target_node_idx,
        source_mark,
        target_mark
    )
}

fn trace_port_origin(port: &LPortRef) -> Option<usize> {
    port.lock_ok()
        .and_then(|mut port_guard| port_guard.get_property(InternalProperties::ORIGIN))
        .and_then(|origin| match origin {
            Origin::ElkPort(origin_id) => Some(origin_id),
            _ => None,
        })
}


fn edge_origin_id(edge: &LEdgeRef) -> Option<usize> {
    edge.lock_ok()
        .and_then(|mut edge_guard| edge_guard.get_property(InternalProperties::ORIGIN))
        .and_then(|origin| match origin {
            Origin::ElkEdge(origin_id) => Some(origin_id),
            _ => None,
        })
}

