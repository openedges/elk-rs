use std::collections::BTreeSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LGraph, LNode, LNodeRef, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

pub struct StretchWidthLayerer {
    width_current: f64,
    width_up: f64,
    max_width: f64,
    upper_layer_influence: f64,
    sorted_layerless_nodes: Vec<LNodeRef>,
    already_placed_nodes: BTreeSet<usize>,
    already_placed_in_other_layers: BTreeSet<usize>,
    temp_layerless_nodes: Vec<LNodeRef>,
    successors: Vec<BTreeSet<usize>>,
    out_degree: Vec<i32>,
    remaining_outgoing: Vec<i32>,
    in_degree: Vec<i32>,
    minimum_node_size: f64,
    maximum_node_size: f64,
    norm_size: Vec<f64>,
    dummy_size: f64,
}

impl StretchWidthLayerer {
    pub fn new() -> Self {
        StretchWidthLayerer {
            width_current: 0.0,
            width_up: 0.0,
            max_width: 0.0,
            upper_layer_influence: 0.0,
            sorted_layerless_nodes: Vec::new(),
            already_placed_nodes: BTreeSet::new(),
            already_placed_in_other_layers: BTreeSet::new(),
            temp_layerless_nodes: Vec::new(),
            successors: Vec::new(),
            out_degree: Vec::new(),
            remaining_outgoing: Vec::new(),
            in_degree: Vec::new(),
            minimum_node_size: f64::INFINITY,
            maximum_node_size: f64::NEG_INFINITY,
            norm_size: Vec::new(),
            dummy_size: 0.0,
        }
    }

    fn compute_sorted_nodes(&mut self, graph: &LGraph) {
        let unsorted_nodes = graph.layerless_nodes().clone();
        self.sorted_layerless_nodes = unsorted_nodes.clone();
        for node in &unsorted_nodes {
            let rank = self.get_rank(node);
            set_node_id(node, rank);
        }
        self.sorted_layerless_nodes
            .sort_by_key(|node| std::cmp::Reverse(node_id(node)));
    }

    fn get_rank(&self, node: &LNodeRef) -> i32 {
        let outgoing_count = node.lock().outgoing_edges().len() as i32;
        let mut max_rank = outgoing_count;
        let incoming = node
            .lock().incoming_edges();
        for edge in incoming {
            let predecessor = edge
                .lock().source()
                .and_then(|port| port.lock().node());
            if let Some(predecessor) = predecessor {
                let pre_out = predecessor.lock().outgoing_edges().len() as i32;
                max_rank = max_rank.max(pre_out);
            }
        }
        max_rank
    }

    fn compute_successors(&mut self) {
        let mut successors: Vec<BTreeSet<usize>> = Vec::new();
        for (index, node) in self.sorted_layerless_nodes.iter().enumerate() {
            set_node_id(node, index as i32);
            let mut out_nodes: BTreeSet<usize> = BTreeSet::new();
            let outgoing = node
                .lock().outgoing_edges();
            for edge in outgoing {
                let target = edge
                    .lock().target()
                    .and_then(|port| port.lock().node());
                if let Some(target) = target {
                    out_nodes.insert(node_id_usize(&target));
                }
            }
            out_nodes.remove(&node_id_usize(node));
            successors.push(out_nodes);
        }
        self.successors = successors;
    }

    fn compute_degrees(&mut self) {
        let count = self.sorted_layerless_nodes.len();
        self.in_degree = vec![0; count];
        self.out_degree = vec![0; count];
        for node in &self.sorted_layerless_nodes {
            let node_index = node_id_usize(node);
            let incoming = node
                .lock().incoming_edges().len() as i32;
            let outgoing = node
                .lock().outgoing_edges().len() as i32;
            self.in_degree[node_index] = incoming;
            self.out_degree[node_index] = outgoing;
        }
    }

    fn min_max_node_size(&mut self) {
        for node in &self.sorted_layerless_nodes {
            let node_type = node
                .lock().node_type();
            if node_type != NodeType::Normal {
                continue;
            }
            let size = node
                .lock().shape().size_ref().y;
            self.minimum_node_size = self.minimum_node_size.min(size);
            self.maximum_node_size = self.maximum_node_size.max(size);
        }
    }

    fn compute_normalized_size(&mut self) {
        let count = self.sorted_layerless_nodes.len();
        self.norm_size = vec![0.0; count];
        for node in &self.sorted_layerless_nodes {
            let node_index = node_id_usize(node);
            let size = node
                .lock().shape().size_ref().y;
            self.norm_size[node_index] = size / self.minimum_node_size;
        }
    }

    fn get_average_out_degree(&self, graph: &LGraph) -> f64 {
        let nodes = graph.layerless_nodes();
        if nodes.is_empty() {
            return 0.0;
        }
        let mut total_out = 0.0;
        for node in nodes {
            let out_count = node
                .lock().outgoing_edges().len() as f64;
            total_out += out_count;
        }
        total_out / nodes.len() as f64
    }

    fn condition_go_up(&self, node: &LNodeRef) -> bool {
        let node_index = node_id_usize(node);
        let out_deg = self.out_degree[node_index] as f64;
        let in_deg = self.in_degree[node_index] as f64;
        let a = (self.width_current - out_deg * self.dummy_size + self.norm_size[node_index])
            > self.max_width;
        let b = (self.width_up + in_deg * self.dummy_size)
            > (self.max_width * self.upper_layer_influence * self.dummy_size);
        a || b
    }

    fn select_node(&self) -> Option<LNodeRef> {
        for node in &self.temp_layerless_nodes {
            let node_index = node_id_usize(node);
            if self.remaining_outgoing[node_index] <= 0 {
                return Some(node.clone());
            }
        }
        None
    }

    fn update_outgoing(&mut self, current_layer: &LayerRef) {
        let nodes = current_layer
            .lock().nodes().clone();
        for node in nodes {
            let incoming = node
                .lock().incoming_edges();
            for edge in incoming {
                let source = edge
                    .lock().source()
                    .and_then(|port| port.lock().node());
                if let Some(source) = source {
                    let pos = node_id_usize(&source);
                    if pos < self.remaining_outgoing.len() {
                        self.remaining_outgoing[pos] -= 1;
                    }
                }
            }
        }
    }
}

impl Default for StretchWidthLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for StretchWidthLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("StretchWidth layering", 1.0);

        if graph.layerless_nodes().is_empty() {
            monitor.done();
            return;
        }

        self.width_current = 0.0;
        self.width_up = 0.0;
        self.minimum_node_size = f64::INFINITY;
        self.maximum_node_size = f64::NEG_INFINITY;

        self.dummy_size = graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE)
            .unwrap_or(0.0);

        self.compute_sorted_nodes(graph);
        self.compute_successors();
        self.compute_degrees();
        self.min_max_node_size();
        self.compute_normalized_size();
        self.minimum_node_size = self.minimum_node_size.max(1.0);
        self.maximum_node_size = self.maximum_node_size.max(1.0);

        self.dummy_size /= self.minimum_node_size;
        self.max_width = self.maximum_node_size / self.minimum_node_size;
        self.upper_layer_influence = self.get_average_out_degree(graph);

        let graph_ref = self
            .sorted_layerless_nodes
            .first()
            .and_then(|node| node.lock().graph())
            .unwrap_or_default();

        let mut current_layer = Layer::new(&graph_ref);
        graph.layers_mut().push(current_layer.clone());

        self.temp_layerless_nodes = self.sorted_layerless_nodes.clone();
        self.remaining_outgoing = self.out_degree.clone();
        self.already_placed_nodes.clear();
        self.already_placed_in_other_layers.clear();

        while !self.temp_layerless_nodes.is_empty() {
            let selected_node = self.select_node();
            if selected_node.is_none()
                || (selected_node
                    .as_ref()
                    .map(|node| self.condition_go_up(node))
                    .unwrap_or(false)
                    && !self.already_placed_nodes.is_empty())
            {
                self.update_outgoing(&current_layer);
                current_layer = Layer::new(&graph_ref);
                graph.layers_mut().push(current_layer.clone());
                self.already_placed_in_other_layers
                    .extend(self.already_placed_nodes.iter().copied());
                self.already_placed_nodes.clear();
                self.width_current = self.width_up;
                self.width_up = 0.0;
            } else if let Some(selected_node) = selected_node {
                if self.condition_go_up(&selected_node) {
                    graph.layers_mut().clear();
                    current_layer = Layer::new(&graph_ref);
                    graph.layers_mut().push(current_layer.clone());
                    self.width_current = 0.0;
                    self.width_up = 0.0;
                    self.already_placed_nodes.clear();
                    self.already_placed_in_other_layers.clear();
                    self.max_width += 1.0;
                    self.temp_layerless_nodes = self.sorted_layerless_nodes.clone();
                    self.remaining_outgoing = self.out_degree.clone();
                } else {
                    LNode::set_layer(&selected_node, Some(current_layer.clone()));
                    if let Some(index) = self
                        .temp_layerless_nodes
                        .iter()
                        .position(|node| Arc::ptr_eq(node, &selected_node))
                    {
                        self.temp_layerless_nodes.remove(index);
                    }
                    let node_index = node_id_usize(&selected_node);
                    self.already_placed_nodes.insert(node_index);
                    self.width_current = self.width_current
                        - (self.out_degree[node_index] as f64) * self.dummy_size
                        + self.norm_size[node_index];
                    self.width_up += (self.in_degree[node_index] as f64) * self.dummy_size;
                }
            }
        }

        graph.layerless_nodes_mut().clear();
        graph.layers_mut().reverse();

        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        _graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
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
        Some(config)
    }
}

fn node_id(node: &LNodeRef) -> i32 {
    node.lock().shape().graph_element().id
}

fn node_id_usize(node: &LNodeRef) -> usize {
    node_id(node) as usize
}

fn set_node_id(node: &LNodeRef, value: i32) {
    {
        let mut node_guard = node.lock();
        node_guard.shape().graph_element().id = value;
    }
}
