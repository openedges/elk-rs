use std::cmp::Ordering;
use std::sync::Arc;
use std::sync::LazyLock;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{Layer, LayerRef, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef};
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

pub struct CoffmanGrahamLayerer {
    node_mark: Vec<bool>,
    edge_mark: Vec<bool>,
    in_deg: Vec<i32>,
    out_deg: Vec<i32>,
    topo_ord: Vec<i32>,
    in_topo: Vec<Vec<i32>>,
}

impl CoffmanGrahamLayerer {
    pub fn new() -> Self {
        CoffmanGrahamLayerer {
            node_mark: Vec::new(),
            edge_mark: Vec::new(),
            in_deg: Vec::new(),
            out_deg: Vec::new(),
            topo_ord: Vec::new(),
            in_topo: Vec::new(),
        }
    }

    fn transitive_reduction(&mut self, nodes: &[LNodeRef]) {
        for start in nodes {
            self.node_mark.fill(false);
            let outgoing = start
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let target = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                if let Some(target) = target {
                    self.dfs(start, &target);
                }
            }
        }
    }

    fn dfs(&mut self, start: &LNodeRef, node: &LNodeRef) {
        let node_index = node_id(node);
        if node_index < self.node_mark.len() && self.node_mark[node_index] {
            return;
        }

        let outgoing = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.outgoing_edges())
            .unwrap_or_default();
        for edge in outgoing {
            let target = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            let Some(target) = target else {
                continue;
            };
            let incoming = target
                .lock()
                .ok()
                .map(|node_guard| node_guard.incoming_edges())
                .unwrap_or_default();
            for transitive in incoming {
                let source = transitive
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.source())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                if let Some(source) = source {
                    if Arc::ptr_eq(&source, start) {
                        let edge_index = edge_id(&transitive);
                        if edge_index < self.edge_mark.len() {
                            self.edge_mark[edge_index] = true;
                        }
                    }
                }
            }
            self.dfs(start, &target);
        }

        if node_index < self.node_mark.len() {
            self.node_mark[node_index] = true;
        }
    }

    fn compare_nodes_in_topo(&self, u: &LNodeRef, v: &LNodeRef) -> Ordering {
        let uid = node_id(u);
        let vid = node_id(v);
        let in_list_u: &[i32] = self.in_topo.get(uid).map(|list| list.as_slice()).unwrap_or(&[]);
        let in_list_v: &[i32] = self.in_topo.get(vid).map(|list| list.as_slice()).unwrap_or(&[]);
        let mut i = in_list_u.len();
        let mut j = in_list_v.len();
        while i > 0 && j > 0 {
            let iu = in_list_u[i - 1];
            let iv = in_list_v[j - 1];
            if iu != iv {
                return iu.cmp(&iv);
            }
            i -= 1;
            j -= 1;
        }
        uid.cmp(&vid)
    }

    fn create_layer(&self, graph_ref: &LGraphRef, layers: &mut Vec<LayerRef>) -> LayerRef {
        let layer = Layer::new(graph_ref);
        layers.push(layer.clone());
        layer
    }

    fn is_layer_full(&self, layer: &LayerRef, bound: i32) -> bool {
        let bound = if bound < 0 { 0 } else { bound as usize };
        layer
            .lock()
            .ok()
            .map(|layer_guard| layer_guard.nodes().len() >= bound)
            .unwrap_or(false)
    }

    fn can_add(&self, node: &LNodeRef, layer: &LayerRef) -> bool {
        let outgoing = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.outgoing_edges())
            .unwrap_or_default();
        for edge in outgoing {
            let target_layer = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.target())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()))
                .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.layer()));
            if let Some(target_layer) = target_layer {
                if Arc::ptr_eq(&target_layer, layer) {
                    return false;
                }
            }
        }
        true
    }
}

impl Default for CoffmanGrahamLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for CoffmanGrahamLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Coffman-Graham Layering", 1.0);

        let nodes = graph.layerless_nodes().clone();
        if nodes.is_empty() {
            monitor.done();
            return;
        }

        let graph_ref = nodes
            .first()
            .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.graph()))
            .unwrap_or_default();

        let bound = graph
            .get_property(LayeredOptions::LAYERING_COFFMAN_GRAHAM_LAYER_BOUND)
            .unwrap_or(i32::MAX);

        let mut edge_index = 0usize;
        for (index, node) in nodes.iter().enumerate() {
            set_node_id(node, index as i32);
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                set_edge_id(&edge, edge_index as i32);
                edge_index += 1;
            }
        }

        let node_count = nodes.len();
        self.node_mark = vec![false; node_count];
        self.edge_mark = vec![false; edge_index];
        self.in_deg = vec![0; node_count];
        self.out_deg = vec![0; node_count];
        self.topo_ord = vec![0; node_count];
        self.in_topo = vec![Vec::new(); node_count];

        self.transitive_reduction(&nodes);

        let mut sources: Vec<LNodeRef> = Vec::new();
        for node in &nodes {
            let incoming = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.incoming_edges())
                .unwrap_or_default();
            let node_index = node_id(node);
            for edge in incoming {
                let edge_index = edge_id(&edge);
                if edge_index < self.edge_mark.len() && !self.edge_mark[edge_index] {
                    self.in_deg[node_index] += 1;
                }
            }
            if self.in_deg[node_index] == 0 {
                sources.push(node.clone());
            }
        }

        let mut topo_index = 0;
        while let Some(node) = pop_best_by(&mut sources, |a, b| self.compare_nodes_in_topo(a, b)) {
            let node_index = node_id(&node);
            self.topo_ord[node_index] = topo_index;
            topo_index += 1;

            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                let edge_index = edge_id(&edge);
                if edge_index < self.edge_mark.len() && self.edge_mark[edge_index] {
                    continue;
                }
                let target = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                let Some(target) = target else {
                    continue;
                };
                let target_index = node_id(&target);
                self.in_deg[target_index] -= 1;
                self.in_topo[target_index].push(self.topo_ord[node_index]);
                if self.in_deg[target_index] == 0 {
                    sources.push(target);
                }
            }
        }

        let mut sinks: Vec<LNodeRef> = Vec::new();
        for node in &nodes {
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            let node_index = node_id(node);
            for edge in outgoing {
                let edge_index = edge_id(&edge);
                if edge_index < self.edge_mark.len() && !self.edge_mark[edge_index] {
                    self.out_deg[node_index] += 1;
                }
            }
            if self.out_deg[node_index] == 0 {
                sinks.push(node.clone());
            }
        }

        let mut layers: Vec<LayerRef> = Vec::new();
        let mut current_layer = self.create_layer(&graph_ref, &mut layers);
        while let Some(node) = pop_best_by(&mut sinks, |a, b| {
            let a_index = node_id(a);
            let b_index = node_id(b);
            self.topo_ord[b_index].cmp(&self.topo_ord[a_index])
        }) {
            if self.is_layer_full(&current_layer, bound) || !self.can_add(&node, &current_layer) {
                current_layer = self.create_layer(&graph_ref, &mut layers);
            }

            LNode::set_layer(&node, Some(current_layer.clone()));

            let incoming = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.incoming_edges())
                .unwrap_or_default();
            for edge in incoming {
                let edge_index = edge_id(&edge);
                if edge_index < self.edge_mark.len() && self.edge_mark[edge_index] {
                    continue;
                }
                let source = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.source())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                let Some(source) = source else {
                    continue;
                };
                let source_index = node_id(&source);
                self.out_deg[source_index] -= 1;
                if self.out_deg[source_index] == 0 {
                    sinks.push(source);
                }
            }
        }

        for layer in layers.iter().rev() {
            graph.layers_mut().push(layer.clone());
        }
        graph.layerless_nodes_mut().clear();

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

fn pop_best_by<T>(items: &mut Vec<T>, cmp: impl Fn(&T, &T) -> Ordering) -> Option<T> {
    if items.is_empty() {
        return None;
    }
    let mut best_index = 0usize;
    for index in 1..items.len() {
        if cmp(&items[index], &items[best_index]) == Ordering::Less {
            best_index = index;
        }
    }
    Some(items.swap_remove(best_index))
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn set_node_id(node: &LNodeRef, value: i32) {
    if let Ok(mut node_guard) = node.lock() {
        node_guard.shape().graph_element().id = value;
    }
}

fn edge_id(edge: &LEdgeRef) -> usize {
    edge.lock()
        .ok()
        .map(|mut edge_guard| edge_guard.graph_element().id as usize)
        .unwrap_or(0)
}

fn set_edge_id(edge: &LEdgeRef, value: i32) {
    if let Ok(mut edge_guard) = edge.lock() {
        edge_guard.graph_element().id = value;
    }
}
