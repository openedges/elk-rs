use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_alg_common::org::eclipse::elk::alg::common::networksimplex::{
    NEdge, NGraph, NNode, NNodeRef, NetworkSimplex,
};
use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNodeRef, Layer};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static BASELINE_PROCESSING_CONFIGURATION: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config
        .add_before(
            LayeredPhases::P1CycleBreaking,
            Arc::new(IntermediateProcessorStrategy::EdgeAndLayerConstraintEdgeReverser),
        )
        .add_before(
            LayeredPhases::P2Layering,
            Arc::new(IntermediateProcessorStrategy::LayerConstraintPreprocessor),
        )
        .add_before(
            LayeredPhases::P3NodeOrdering,
            Arc::new(IntermediateProcessorStrategy::LayerConstraintPostprocessor),
        );
    config
});

const ITER_LIMIT_FACTOR: i32 = 4;

pub struct NetworkSimplexLayerer {
    node_visited: Vec<bool>,
    component_nodes: Vec<LNodeRef>,
}

impl NetworkSimplexLayerer {
    pub fn new() -> Self {
        NetworkSimplexLayerer {
            node_visited: Vec::new(),
            component_nodes: Vec::new(),
        }
    }

    fn connected_components(&mut self, nodes: &[LNodeRef]) -> Vec<Vec<LNodeRef>> {
        if self.node_visited.len() < nodes.len() {
            self.node_visited = vec![false; nodes.len()];
        } else {
            self.node_visited.fill(false);
        }
        self.component_nodes.clear();

        for (index, node) in nodes.iter().enumerate() {
            if let Ok(mut node_guard) = node.lock() {
                node_guard.shape().graph_element().id = index as i32;
            }
        }

        let mut components: Vec<Vec<LNodeRef>> = Vec::new();
        for node in nodes {
            let idx = node_index(node);
            if idx < self.node_visited.len() && !self.node_visited[idx] {
                self.connected_components_dfs(node);
                if components.is_empty() || components[0].len() < self.component_nodes.len() {
                    components.insert(0, self.component_nodes.clone());
                } else {
                    components.push(self.component_nodes.clone());
                }
                self.component_nodes.clear();
            }
        }
        components
    }

    fn connected_components_dfs(&mut self, node: &LNodeRef) {
        let idx = node_index(node);
        if idx >= self.node_visited.len() || self.node_visited[idx] {
            return;
        }
        self.node_visited[idx] = true;
        self.component_nodes.push(node.clone());

        // Collect opposite nodes: clone ports Vec to release node lock early,
        // then iterate edges via incoming/outgoing directly (avoids connected_edges() alloc)
        let ports = match node.lock() {
            Ok(node_guard) => node_guard.ports().clone(),
            Err(_) => Vec::new(),
        };

        let mut opposite_nodes: Vec<LNodeRef> = Vec::new();
        for port in &ports {
            if let Ok(port_guard) = port.lock() {
                for edge in port_guard.incoming_edges() {
                    if let Some(src_node) = edge
                        .lock()
                        .ok()
                        .and_then(|e| e.source())
                        .and_then(|p| p.lock().ok().and_then(|pg| pg.node()))
                    {
                        opposite_nodes.push(src_node);
                    }
                }
                for edge in port_guard.outgoing_edges() {
                    if let Some(tgt_node) = edge
                        .lock()
                        .ok()
                        .and_then(|e| e.target())
                        .and_then(|p| p.lock().ok().and_then(|pg| pg.node()))
                    {
                        opposite_nodes.push(tgt_node);
                    }
                }
            }
        }
        for opp in opposite_nodes {
            self.connected_components_dfs(&opp);
        }
    }

    fn initialize_graph(&self, nodes: &[LNodeRef]) -> NGraph {
        let mut graph = NGraph::new();
        let mut node_map: FxHashMap<usize, NNodeRef> = FxHashMap::default();

        for node in nodes {
            let origin: Arc<dyn std::any::Any + Send + Sync> = Arc::new(node.clone());
            let nnode = NNode::of().origin(origin).create(&mut graph);
            node_map.insert(Arc::as_ptr(node) as usize, nnode);
        }

        for node in nodes {
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
                let source_node = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.source())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                let target_node = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                let (Some(source_node), Some(target_node)) = (source_node, target_node) else {
                    continue;
                };
                let source_nnode = match node_map.get(&(Arc::as_ptr(&source_node) as usize)) {
                    Some(node) => node.clone(),
                    None => continue,
                };
                let target_nnode = match node_map.get(&(Arc::as_ptr(&target_node) as usize)) {
                    Some(node) => node.clone(),
                    None => continue,
                };

                let priority = edge
                    .lock()
                    .ok()
                    .and_then(|mut edge_guard| {
                        edge_guard.get_property(LayeredOptions::PRIORITY_SHORTNESS)
                    })
                    .unwrap_or(1);
                let weight = (priority.max(1)) as f64;
                let origin: Arc<dyn std::any::Any + Send + Sync> = Arc::new(edge.clone());
                NEdge::of_origin(origin)
                    .weight(weight)
                    .delta(1)
                    .source(source_nnode)
                    .target(target_nnode)
                    .create();
            }
        }

        graph
    }
}

impl Default for NetworkSimplexLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for NetworkSimplexLayerer {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Network simplex layering", 1.0);

        let thoroughness = layered_graph
            .get_property(LayeredOptions::THOROUGHNESS)
            .unwrap_or(7)
            * ITER_LIMIT_FACTOR;
        let nodes = layered_graph.layerless_nodes().clone();
        if nodes.is_empty() {
            monitor.done();
            return;
        }

        let connected_components = self.connected_components(&nodes);
        let mut previous_layering_node_counts: Option<Vec<i32>> = None;

        for (idx, component) in connected_components.iter().enumerate() {
            // Java parity: (int) Math.sqrt(connComp.size()) -> floor(sqrt(n)) for n >= 1
            let iter_limit = thoroughness * (component.len() as f64).sqrt() as i32;
            let mut graph = self.initialize_graph(component);

            let mut simplex = NetworkSimplex::for_graph(&mut graph);
            simplex.with_iteration_limit(iter_limit);
            simplex.with_previous_layering(previous_layering_node_counts.as_deref());
            simplex.with_balancing(true);

            let mut sub_monitor = monitor.sub_task(1.0 / connected_components.len() as f32);
            simplex.execute_with_monitor(sub_monitor.as_mut());

            for nnode in &graph.nodes {
                let (layer, origin) = match nnode.lock() {
                    Ok(node_guard) => (node_guard.layer, node_guard.origin.clone()),
                    Err(_) => continue,
                };
                let Some(origin) = origin else {
                    continue;
                };
                let Some(l_node) = origin.as_ref().downcast_ref::<LNodeRef>() else {
                    continue;
                };

                while layered_graph.layers().len() <= layer as usize {
                    let graph_ref = l_node
                        .lock()
                        .ok()
                        .and_then(|node_guard| node_guard.graph())
                        .unwrap_or_default();
                    layered_graph.layers_mut().push(Layer::new(&graph_ref));
                }

                if let Some(layer_ref) = layered_graph.layers().get(layer as usize).cloned() {
                    crate::org::eclipse::elk::alg::layered::graph::LNode::set_layer(
                        l_node,
                        Some(layer_ref),
                    );
                }
            }

            if connected_components.len() > 1 {
                let mut counts = vec![0i32; layered_graph.layers().len()];
                for (layer_idx, layer) in layered_graph.layers().iter().enumerate() {
                    if let Ok(layer_guard) = layer.lock() {
                        counts[layer_idx] = layer_guard.nodes().len() as i32;
                    }
                }
                previous_layering_node_counts = Some(counts);
            }

            if idx + 1 == connected_components.len() {
                previous_layering_node_counts = None;
            }
        }

        layered_graph.layerless_nodes_mut().clear();
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        Some(LayoutProcessorConfiguration::create_from(
            &BASELINE_PROCESSING_CONFIGURATION,
        ))
    }
}

fn node_index(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}
