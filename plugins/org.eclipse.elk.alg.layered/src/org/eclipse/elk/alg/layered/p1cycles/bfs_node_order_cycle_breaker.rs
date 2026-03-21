use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef, NodeRefKey};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GroupOrderStrategy, InternalProperties, LayeredOptions,
};
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

pub struct BfsNodeOrderCycleBreaker {
    sources: BTreeSet<NodeRefKey>,
    sinks: BTreeSet<NodeRefKey>,
    visited: Vec<bool>,
    bfs_queue: VecDeque<LNodeRef>,
    edges_to_be_reversed: Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
}

impl BfsNodeOrderCycleBreaker {
    pub fn new() -> Self {
        BfsNodeOrderCycleBreaker {
            sources: BTreeSet::new(),
            sinks: BTreeSet::new(),
            visited: Vec::new(),
            bfs_queue: VecDeque::new(),
            edges_to_be_reversed: Vec::new(),
        }
    }

    fn bfs_loop(&mut self, graph: &mut LGraph) {
        while let Some(node) = self.bfs_queue.pop_front() {
            self.bfs(&node, graph);
        }
    }

    fn bfs(&mut self, node: &LNodeRef, graph: &mut LGraph) {
        let index = node_index(node);
        if index >= self.visited.len() || self.visited[index] {
            return;
        }
        self.visited[index] = true;

        let enforce_group_model_order = graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY)
            .unwrap_or(GroupOrderStrategy::OnlyWithinGroup)
            == GroupOrderStrategy::Enforced;
        let max_model_order_nodes = graph
            .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
            .unwrap_or(0);

        let outgoing_edges = node
            .lock_ok()
            .map(|node_guard| node_guard.outgoing_edges())
            .unwrap_or_default();

        let mut model_order_map: BTreeMap<
            i32,
            Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
        > = BTreeMap::new();
        for edge in outgoing_edges {
            let target_node = edge
                .lock_ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|target| {
                    target
                        .lock_ok()
                        .and_then(|target_guard| target_guard.node())
                });
            let Some(target_node) = target_node else {
                continue;
            };

            let has_model_order = target_node.lock_ok().is_some_and(|mut node_guard| {
                node_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(InternalProperties::MODEL_ORDER)
            });

            if !has_model_order {
                let key = i32::MAX - model_order_map.len() as i32;
                model_order_map.entry(key).or_default().push(edge);
                continue;
            }

            let target_model_order = if enforce_group_model_order {
                let (group_id, model_order) = target_node
                    .lock_ok()
                    .map(|mut node_guard| {
                        (
                            node_guard
                                .get_property(LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
                                .unwrap_or(0),
                            node_guard
                                .get_property(InternalProperties::MODEL_ORDER)
                                .unwrap_or(0),
                        )
                    })
                    .unwrap_or((0, 0));
                max_model_order_nodes * group_id + model_order
            } else {
                target_node
                    .lock_ok()
                    .and_then(|mut node_guard| {
                        node_guard.get_property(InternalProperties::MODEL_ORDER)
                    })
                    .unwrap_or(0)
            };

            model_order_map
                .entry(target_model_order)
                .or_default()
                .push(edge);
        }

        for edges in model_order_map.values() {
            let Some(representative) = edges.first() else {
                continue;
            };

            let is_self_loop = representative
                .lock_ok()
                .map(|edge_guard| edge_guard.is_self_loop())
                .unwrap_or(false);
            if is_self_loop {
                continue;
            }

            let target_node = representative
                .lock_ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|target| {
                    target
                        .lock_ok()
                        .and_then(|target_guard| target_guard.node())
                });
            let Some(target_node) = target_node else {
                continue;
            };

            let target_index = node_index(&target_node);
            let node_is_source = self.sources.contains(&NodeRefKey(node.clone()));
            let target_is_sink = self.sinks.contains(&NodeRefKey(target_node.clone()));
            if target_index < self.visited.len()
                && self.visited[target_index]
                && !node_is_source
                && !target_is_sink
            {
                self.edges_to_be_reversed.extend(edges.iter().cloned());
            } else {
                self.bfs_queue.push_back(target_node);
            }
        }
    }
}

impl Default for BfsNodeOrderCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for BfsNodeOrderCycleBreaker {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Breadth-first cycle removal", 1.0);

        let nodes = graph.layerless_nodes().clone();
        self.visited = vec![false; nodes.len()];
        self.sources.clear();
        self.sinks.clear();
        self.bfs_queue.clear();
        self.edges_to_be_reversed.clear();

        let mut sources_in_order: Vec<LNodeRef> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let (is_source, is_sink) = match node.lock_ok() {
            Some(mut node_guard) => {
                    node_guard.shape().graph_element().id = index as i32;
                    (
                        node_guard.incoming_edges().is_empty(),
                        node_guard.outgoing_edges().is_empty(),
                    )
                }
            None => (false, false),
            };

            if is_source {
                self.sources.insert(NodeRefKey(node.clone()));
                sources_in_order.push(node.clone());
            }
            if is_sink {
                self.sinks.insert(NodeRefKey(node.clone()));
            }
        }

        for source in sources_in_order {
            self.bfs_queue.push_back(source);
            self.bfs_loop(graph);
        }
        self.bfs_loop(graph);

        let mut changed = true;
        while changed {
            changed = false;
            for (i, visited) in self.visited.iter().enumerate() {
                if !visited {
                    self.bfs_queue.push_back(nodes[i].clone());
                    changed = true;
                    break;
                }
            }
            self.bfs_loop(graph);
        }

        let dummy_graph = LGraph::new();
        for edge in self.edges_to_be_reversed.drain(..) {
            LEdge::reverse(&edge, &dummy_graph, true);
            graph.set_property(InternalProperties::CYCLIC, Some(true));
        }

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
