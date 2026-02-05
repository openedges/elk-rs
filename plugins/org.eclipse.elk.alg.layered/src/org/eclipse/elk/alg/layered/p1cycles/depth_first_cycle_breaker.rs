use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LEdge, LGraph, LNodeRef};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
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

pub struct DepthFirstCycleBreaker {
    visited: Vec<bool>,
    active: Vec<bool>,
    edges_to_reverse: Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef>,
}

impl DepthFirstCycleBreaker {
    pub fn new() -> Self {
        DepthFirstCycleBreaker {
            visited: Vec::new(),
            active: Vec::new(),
            edges_to_reverse: Vec::new(),
        }
    }

    fn dfs(&mut self, node: &LNodeRef) {
        let index = node_index(node);
        if index >= self.visited.len() || self.visited[index] {
            return;
        }

        self.visited[index] = true;
        self.active[index] = true;

        let outgoing = match node.lock() {
            Ok(node_guard) => node_guard.outgoing_edges(),
            Err(_) => Vec::new(),
        };

        for edge in outgoing {
            let is_self_loop = edge
                .lock()
                .map(|edge_guard| edge_guard.is_self_loop())
                .unwrap_or(false);
            if is_self_loop {
                continue;
            }

            let target_node = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            let Some(target_node) = target_node else {
                continue;
            };
            let target_index = node_index(&target_node);
            if target_index < self.active.len() && self.active[target_index] {
                self.edges_to_reverse.push(edge);
            } else {
                self.dfs(&target_node);
            }
        }

        self.active[index] = false;
    }
}

impl Default for DepthFirstCycleBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for DepthFirstCycleBreaker {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Depth-first cycle removal", 1.0);

        let nodes = graph.layerless_nodes().clone();
        let node_count = nodes.len();

        self.visited = vec![false; node_count];
        self.active = vec![false; node_count];
        self.edges_to_reverse.clear();

        let mut sources: Vec<LNodeRef> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.shape().graph_element().id = index as i32;
                if node_guard.incoming_edges().is_empty() {
                    sources.push(node.clone());
                }
            }
        }

        for source in &sources {
            self.dfs(source);
        }

        for node in &nodes {
            let index = node_index(node);
            if index < node_count && !self.visited[index] {
                self.dfs(node);
            }
        }

        let dummy_graph = crate::org::eclipse::elk::alg::layered::graph::LGraph::new();
        for edge in self.edges_to_reverse.drain(..) {
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
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}
