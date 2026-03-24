use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef};
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

pub struct DfsNodeOrderCycleBreaker {
    visited: Vec<bool>,
    active: Vec<bool>,
    edges_to_be_reversed: Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
}

impl DfsNodeOrderCycleBreaker {
    pub fn new() -> Self {
        DfsNodeOrderCycleBreaker {
            visited: Vec::new(),
            active: Vec::new(),
            edges_to_be_reversed: Vec::new(),
        }
    }

    fn dfs(&mut self, node: &LNodeRef, graph: &mut LGraph) {
        let index = node_index(node);
        if index >= self.visited.len() || self.visited[index] {
            return;
        }

        self.visited[index] = true;
        self.active[index] = true;

        let enforce_group_model_order = graph
            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CB_GROUP_ORDER_STRATEGY)
            .unwrap_or(GroupOrderStrategy::OnlyWithinGroup)
            == GroupOrderStrategy::Enforced;
        let max_model_order_nodes = graph
            .get_property(InternalProperties::MAX_MODEL_ORDER_NODES)
            .unwrap_or(0);

        let outgoing_edges = {
            let node_guard = node.lock();
            node_guard.outgoing_edges()
        };

        let mut model_order_map: BTreeMap<
            i32,
            Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
        > = BTreeMap::new();
        for edge in outgoing_edges {
            let target_node = {
                let edge_guard = edge.lock();
                edge_guard.target()
                    .and_then(|target| {
                        let target_guard = target.lock();
                        target_guard.node()
                    })
            };
            let Some(target_node) = target_node else {
                continue;
            };

            let has_model_order = {
                let mut node_guard = target_node.lock();
                node_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(InternalProperties::MODEL_ORDER)
            };

            if !has_model_order {
                let key = i32::MAX - model_order_map.len() as i32;
                model_order_map.entry(key).or_default().push(edge);
                continue;
            }

            let target_model_order = if enforce_group_model_order {
                let (group_id, model_order) = {
                    let node_guard = target_node.lock();
                    (
                        node_guard
                            .get_property(LayeredOptions::GROUP_MODEL_ORDER_CYCLE_BREAKING_ID)
                            .unwrap_or(0),
                        node_guard
                            .get_property(InternalProperties::MODEL_ORDER)
                            .unwrap_or(0),
                    )
                };
                max_model_order_nodes * group_id + model_order
            } else {
                let node_guard = target_node.lock();
                node_guard.get_property(InternalProperties::MODEL_ORDER)
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

            let is_self_loop = {
                let edge_guard = representative.lock();
                edge_guard.is_self_loop()
            };
            if is_self_loop {
                continue;
            }

            let target_node = {
                let edge_guard = representative.lock();
                edge_guard.target()
                    .and_then(|target| {
                        let target_guard = target.lock();
                        target_guard.node()
                    })
            };
            let Some(target_node) = target_node else {
                continue;
            };

            let target_index = node_index(&target_node);
            if target_index < self.active.len() && self.active[target_index] {
                self.edges_to_be_reversed.extend(edges.iter().cloned());
            } else {
                self.dfs(&target_node, graph);
            }
        }

        self.active[index] = false;
    }
}

impl Default for DfsNodeOrderCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for DfsNodeOrderCycleBreaker {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Depth-first cycle removal", 1.0);

        let nodes = graph.layerless_nodes().clone();
        self.visited = vec![false; nodes.len()];
        self.active = vec![false; nodes.len()];
        self.edges_to_be_reversed.clear();

        let mut sources: Vec<LNodeRef> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let is_source = {
                let mut node_guard = node.lock();
                node_guard.shape().graph_element().id = index as i32;
                node_guard.incoming_edges().is_empty()
            };
            if is_source {
                sources.push(node.clone());
            }
        }

        for source in &sources {
            self.dfs(source, graph);
        }

        for node in &nodes {
            let index = node_index(node);
            if index < self.visited.len() && !self.visited[index] {
                self.dfs(node, graph);
            }
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
    let mut node_guard = node.lock();
    node_guard.shape().graph_element().id as usize
}
