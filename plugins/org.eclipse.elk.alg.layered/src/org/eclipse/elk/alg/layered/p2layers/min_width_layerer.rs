use std::collections::BTreeSet;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::pair::Pair;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{LGraph, LNode, LNodeRef, Layer, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::LayeredOptions;
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

const UPPERBOUND_ON_WIDTH_RANGE: (i32, i32) = (1, 4);
const COMPENSATOR_RANGE: (i32, i32) = (1, 2);

pub struct MinWidthLayerer {
    dummy_size: f64,
    minimum_node_size: f64,
    norm_size: Vec<f64>,
    avg_size: f64,
    in_degree: Vec<i32>,
    out_degree: Vec<i32>,
}

impl MinWidthLayerer {
    pub fn new() -> Self {
        MinWidthLayerer {
            dummy_size: 0.0,
            minimum_node_size: f64::INFINITY,
            norm_size: Vec::new(),
            avg_size: 0.0,
            in_degree: Vec::new(),
            out_degree: Vec::new(),
        }
    }

    fn precalc_successors(&self, nodes: &[LNodeRef]) -> Vec<BTreeSet<usize>> {
        let mut successors: Vec<BTreeSet<usize>> = Vec::with_capacity(nodes.len());
        for node in nodes {
            let mut out_nodes: BTreeSet<usize> = BTreeSet::new();
            let outgoing = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.outgoing_edges())
                .unwrap_or_default();
            for edge in outgoing {
                if edge
                    .lock()
                    .ok()
                    .map(|edge_guard| edge_guard.is_self_loop())
                    .unwrap_or(false)
                {
                    continue;
                }
                let target = edge
                    .lock()
                    .ok()
                    .and_then(|edge_guard| edge_guard.target())
                    .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
                if let Some(target) = target {
                    out_nodes.insert(node_id_usize(&target));
                }
            }
            successors.push(out_nodes);
        }
        successors
    }

    fn compute_min_width_layering(
        &self,
        upper_bound_on_width: i32,
        compensator: i32,
        nodes: &[LNodeRef],
        node_successors: &[BTreeSet<usize>],
    ) -> Pair<f64, Vec<Vec<LNodeRef>>> {
        let mut layers: Vec<Vec<LNodeRef>> = Vec::new();
        let mut unplaced_nodes: Vec<LNodeRef> = nodes.to_vec();

        let ubw_consider_size = upper_bound_on_width as f64 * self.avg_size;

        let mut out_deg = 0;

        let mut already_placed_in_current_layer: BTreeSet<usize> = BTreeSet::new();
        let mut already_placed_in_other_layers: BTreeSet<usize> = BTreeSet::new();
        let mut current_layer: Vec<LNodeRef> = Vec::new();

        let mut width_current = 0.0;
        let mut width_up = 0.0;
        let mut max_width: f64 = 0.0;
        let mut real_width = 0.0;
        let mut current_spanning_edges = 0.0;
        let mut going_out_from_this_layer = 0.0;

        while !unplaced_nodes.is_empty() {
            let selected_index = select_node(
                &unplaced_nodes,
                node_successors,
                &already_placed_in_other_layers,
            );
            let selected_node = selected_index.map(|index| unplaced_nodes.remove(index));

            if let Some(ref current_node) = selected_node {
                let node_index = node_id_usize(current_node);
                current_layer.push(current_node.clone());
                already_placed_in_current_layer.insert(node_index);

                out_deg = self.out_degree[node_index];
                width_current += self.norm_size[node_index] - (out_deg as f64) * self.dummy_size;

                let in_deg = self.in_degree[node_index];
                width_up += (in_deg as f64) * self.dummy_size;

                going_out_from_this_layer += (out_deg as f64) * self.dummy_size;
                real_width += self.norm_size[node_index];
            }

            let should_go_up = if let Some(current_node) = selected_node.as_ref() {
                if unplaced_nodes.is_empty() {
                    true
                } else {
                    let node_index = node_id_usize(current_node);
                    (width_current >= ubw_consider_size
                        && self.norm_size[node_index] > (out_deg as f64) * self.dummy_size)
                        || width_up >= (compensator as f64) * ubw_consider_size
                }
            } else {
                true
            };

            if should_go_up {
                layers.push(current_layer);
                current_layer = Vec::new();
                already_placed_in_other_layers
                    .extend(already_placed_in_current_layer.iter().copied());
                already_placed_in_current_layer.clear();

                current_spanning_edges -= going_out_from_this_layer;
                max_width = max_width.max(current_spanning_edges * self.dummy_size + real_width);
                current_spanning_edges += width_up;

                width_current = width_up;
                width_up = 0.0;
                going_out_from_this_layer = 0.0;
                real_width = 0.0;
            }
        }

        Pair::of(max_width, layers)
    }
}

impl Default for MinWidthLayerer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for MinWidthLayerer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("MinWidth layering", 1.0);

        let mut not_inserted = graph.layerless_nodes().clone();
        if not_inserted.is_empty() {
            monitor.done();
            return;
        }

        let upper_bound_on_width = graph
            .get_property(LayeredOptions::LAYERING_MIN_WIDTH_UPPER_BOUND_ON_WIDTH)
            .unwrap_or(4);
        let compensator = graph
            .get_property(LayeredOptions::LAYERING_MIN_WIDTH_UPPER_LAYER_ESTIMATION_SCALING_FACTOR)
            .unwrap_or(2);

        self.dummy_size = graph
            .get_property(LayeredOptions::SPACING_EDGE_EDGE)
            .unwrap_or(0.0);

        self.minimum_node_size = f64::INFINITY;
        for node in &not_inserted {
            let node_type = node
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type())
                .unwrap_or(NodeType::Normal);
            if node_type != NodeType::Normal {
                continue;
            }
            let size = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.shape().size_ref().y)
                .unwrap_or(0.0);
            self.minimum_node_size = self.minimum_node_size.min(size);
        }
        self.minimum_node_size = self.minimum_node_size.max(1.0);

        let num_nodes = not_inserted.len();
        self.in_degree = vec![0; num_nodes];
        self.out_degree = vec![0; num_nodes];
        self.norm_size = vec![0.0; num_nodes];
        self.avg_size = 0.0;

        for (index, node) in not_inserted.iter().enumerate() {
            set_node_id(node, index as i32);
            self.in_degree[index] = count_edges_except_self_loops(node, true);
            self.out_degree[index] = count_edges_except_self_loops(node, false);
            let size = node
                .lock()
                .ok()
                .map(|mut node_guard| node_guard.shape().size_ref().y)
                .unwrap_or(0.0);
            self.norm_size[index] = size / self.minimum_node_size;
            self.avg_size += self.norm_size[index];
        }

        self.dummy_size /= self.minimum_node_size;
        self.avg_size /= num_nodes as f64;

        let node_successors = self.precalc_successors(&not_inserted);

        not_inserted.sort_by(|a, b| {
            let out_a = self.out_degree[node_id_usize(a)];
            let out_b = self.out_degree[node_id_usize(b)];
            out_a.cmp(&out_b)
        });
        not_inserted.reverse();

        let mut min_width = f64::INFINITY;
        let mut min_num_layers = i32::MAX;
        let mut candidate_layering: Option<Vec<Vec<LNodeRef>>> = None;

        let mut ubw_start = upper_bound_on_width;
        let mut ubw_end = upper_bound_on_width;
        let mut c_start = compensator;
        let mut c_end = compensator;

        if upper_bound_on_width < 0 {
            ubw_start = UPPERBOUND_ON_WIDTH_RANGE.0;
            ubw_end = UPPERBOUND_ON_WIDTH_RANGE.1;
        }
        if compensator < 0 {
            c_start = COMPENSATOR_RANGE.0;
            c_end = COMPENSATOR_RANGE.1;
        }

        for ubw in ubw_start..=ubw_end {
            for c in c_start..=c_end {
                let result =
                    self.compute_min_width_layering(ubw, c, &not_inserted, &node_successors);
                let new_width = result.first;
                let layering = result.second;

                let new_num_layers = layering.len() as i32;
                if new_width < min_width
                    || (new_width == min_width && new_num_layers < min_num_layers)
                {
                    min_width = new_width;
                    min_num_layers = new_num_layers;
                    candidate_layering = Some(layering);
                }
            }
        }

        let graph_ref = not_inserted
            .first()
            .and_then(|node| node.lock().ok().and_then(|node_guard| node_guard.graph()))
            .unwrap_or_default();

        if let Some(candidate_layering) = candidate_layering {
            for layer_list in candidate_layering {
                let current_layer = Layer::new(&graph_ref);
                for node in layer_list {
                    LNode::set_layer(&node, Some(current_layer.clone()));
                }
                graph.layers_mut().push(current_layer);
            }
        }

        graph.layers_mut().reverse();
        graph.layerless_nodes_mut().clear();

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

fn select_node(
    nodes: &[LNodeRef],
    successors: &[BTreeSet<usize>],
    targets: &BTreeSet<usize>,
) -> Option<usize> {
    for (index, node) in nodes.iter().enumerate() {
        let node_index = node_id_usize(node);
        if successors
            .get(node_index)
            .map(|succ| succ.iter().all(|succ_id| targets.contains(succ_id)))
            .unwrap_or(false)
        {
            return Some(index);
        }
    }
    None
}

fn count_edges_except_self_loops(node: &LNodeRef, incoming: bool) -> i32 {
    let edges = node
        .lock()
        .ok()
        .map(|node_guard| {
            if incoming {
                node_guard.incoming_edges()
            } else {
                node_guard.outgoing_edges()
            }
        })
        .unwrap_or_default();
    let mut count = 0;
    for edge in edges {
        if edge
            .lock()
            .ok()
            .map(|edge_guard| edge_guard.is_self_loop())
            .unwrap_or(false)
        {
            continue;
        }
        count += 1;
    }
    count
}

fn node_id_usize(node: &LNodeRef) -> usize {
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
