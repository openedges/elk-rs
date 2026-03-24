use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, Layer, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::intermediate::breaking_point_info::BreakingPointInfo;
use crate::org::eclipse::elk::alg::layered::intermediate::single_edge_graph_wrapper::GraphStats;
use crate::org::eclipse::elk::alg::layered::intermediate::SingleEdgeGraphWrapper;
use crate::org::eclipse::elk::alg::layered::options::{
    CuttingStrategy, InternalProperties, LayeredOptions, ValidifyStrategy,
};

pub struct BreakingPointInserter;

impl ILayoutProcessor<LGraph> for BreakingPointInserter {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Breaking Point Insertion", 1.0);

        if graph.layers().is_empty() {
            progress_monitor.done();
            return;
        }

        let graph_stats = GraphStats::new(graph);

        let cutting_strategy = graph
            .get_property(LayeredOptions::WRAPPING_CUTTING_STRATEGY)
            .unwrap_or(CuttingStrategy::Msd);
        let calculator: Box<dyn CutIndexCalculator> = match cutting_strategy {
            CuttingStrategy::Manual => Box::new(ManualCutIndexCalculator),
            CuttingStrategy::Ard => Box::new(ARDCutIndexHeuristic),
            CuttingStrategy::Msd => Box::new(MSDCutIndexHeuristic),
        };

        let mut cuts = calculator.get_cut_indexes(graph, &graph_stats);

        if graph
            .get_property(LayeredOptions::WRAPPING_MULTI_EDGE_IMPROVE_CUTS)
            .unwrap_or(true)
        {
            cuts = self.improve_cuts(graph, cuts);
        }

        if !calculator.guarantee_valid()
            && graph
                .graph_element()
                .properties()
                .has_property(LayeredOptions::WRAPPING_VALIDIFY_STRATEGY)
        {
            let validify = graph
                .get_property(LayeredOptions::WRAPPING_VALIDIFY_STRATEGY)
                .unwrap_or(ValidifyStrategy::Greedy);
            cuts = match validify {
                ValidifyStrategy::LookBack => {
                    SingleEdgeGraphWrapper::validify_indexes_looking_back(&graph_stats, cuts)
                }
                ValidifyStrategy::Greedy => {
                    SingleEdgeGraphWrapper::validify_indexes_greedily(&graph_stats, cuts)
                }
                ValidifyStrategy::No => cuts,
            };
        }

        if cuts.is_empty() {
            progress_monitor.done();
            return;
        }

        let _ = self.apply_cuts(graph, &cuts);

        progress_monitor.done();
    }
}

impl BreakingPointInserter {
    fn apply_cuts(&self, graph: &mut LGraph, cuts: &[i32]) -> i32 {
        if cuts.is_empty() {
            return 0;
        }

        let graph_ref = graph_ref_for(graph);
        let mut idx = 0usize;
        let mut cut_index = 0usize;
        let mut next_cut = cuts[cut_index] as usize;
        let mut no_split_edges = 0;

        let mut already_split: Vec<LEdgeRef> = Vec::new();
        let mut open_edges: Vec<LEdgeRef> = Vec::new();

        while idx < graph.layers().len() {
            let layer = graph.layers()[idx].clone();
            let nodes = layer
                .lock().nodes().clone();

            for node in nodes {
                let outgoing = node
                    .lock().outgoing_edges();
                for edge in outgoing {
                    if !contains_edge(&open_edges, &edge) {
                        open_edges.push(edge);
                    }
                }

                let incoming = node
                    .lock().incoming_edges();
                for edge in incoming {
                    remove_edge(&mut open_edges, &edge);
                }
            }

            if idx + 1 == next_cut {
                let bp_layer1 = Layer::new(&graph_ref);
                graph.layers_mut().insert(idx + 1, bp_layer1.clone());
                let bp_layer2 = Layer::new(&graph_ref);
                graph.layers_mut().insert(idx + 2, bp_layer2.clone());

                for original_edge in open_edges.clone() {
                    if !contains_edge(&already_split, &original_edge) {
                        no_split_edges += 1;
                        already_split.push(original_edge.clone());
                    }

                    let (bp_start_marker, in_port_bp1, out_port_bp1) =
                        create_breaking_point_dummy(&graph_ref, &bp_layer1);
                    let (bp_end_marker, in_port_bp2, out_port_bp2) =
                        create_breaking_point_dummy(&graph_ref, &bp_layer2);

                    let model_order = original_edge.lock().get_property(InternalProperties::MODEL_ORDER);

                    let source_port = original_edge
                        .lock().source();
                    let node_start_edge = LEdge::new();
                    LEdge::set_source(&node_start_edge, source_port);
                    LEdge::set_target(&node_start_edge, Some(in_port_bp1));
                    {
                        let mut node_start_edge_guard = node_start_edge.lock();
                        node_start_edge_guard
                            .set_property(InternalProperties::MODEL_ORDER, model_order);
                    }

                    let start_end_edge = LEdge::new();
                    LEdge::set_source(&start_end_edge, Some(out_port_bp1));
                    LEdge::set_target(&start_end_edge, Some(in_port_bp2));
                    {
                        let mut start_end_edge_guard = start_end_edge.lock();
                        start_end_edge_guard
                            .set_property(InternalProperties::MODEL_ORDER, model_order);
                    }

                    LEdge::set_source(&original_edge, Some(out_port_bp2));

                    let bp_info = BreakingPointInfo::new(
                        bp_start_marker.clone(),
                        bp_end_marker.clone(),
                        node_start_edge.clone(),
                        start_end_edge,
                        original_edge,
                    );
                    {
                        let mut start_guard = bp_start_marker.lock();
                        start_guard.set_property(
                            InternalProperties::BREAKING_POINT_INFO,
                            Some(bp_info.clone()),
                        );
                    }
                    {
                        let mut end_guard = bp_end_marker.lock();
                        end_guard.set_property(
                            InternalProperties::BREAKING_POINT_INFO,
                            Some(bp_info.clone()),
                        );
                    }

                    let prev_node = node_start_edge
                        .lock().source()
                        .and_then(|port| port.lock().node());

                    if let Some(prev_node) = prev_node {
                        let prev_is_breaking_point = prev_node.lock().node_type() == NodeType::BreakingPoint;
                        if prev_is_breaking_point {
                            let prev_info = prev_node.lock().get_property(InternalProperties::BREAKING_POINT_INFO);
                            if let Some(prev_info) = prev_info {
                                {
                                    let mut prev_info_guard = prev_info.lock();
                                    prev_info_guard.next = Some(bp_info.clone());
                                }
                                {
                                    let mut bp_info_guard = bp_info.lock();
                                    bp_info_guard.prev = Some(prev_info);
                                }
                            }
                        }
                    }
                }

                cut_index += 1;
                if cut_index >= cuts.len() {
                    break;
                }
                next_cut = cuts[cut_index] as usize;
            }

            idx += 1;
        }

        no_split_edges
    }

    fn improve_cuts(&self, graph: &mut LGraph, cuts: Vec<i32>) -> Vec<i32> {
        if cuts.is_empty() {
            return Vec::new();
        }

        let mut improved = Vec::new();

        let mut cut_states: Vec<CutState> = Vec::with_capacity(cuts.len());
        for (idx, cut_index) in cuts.iter().enumerate() {
            cut_states.push(CutState {
                index: *cut_index,
                new_index: *cut_index,
                prev: if idx > 0 { Some(idx - 1) } else { None },
                suc: None,
                assigned: false,
            });
        }
        for idx in 0..cut_states.len().saturating_sub(1) {
            cut_states[idx].suc = Some(idx + 1);
        }

        let spans = compute_edge_spans(graph);

        for _ in 0..cut_states.len() {
            let mut l_cut: Option<usize> = None;
            let mut r_cut = cut_self_or_next(&cut_states, Some(0));

            let mut best_cut: Option<usize> = None;
            let mut best_score = f64::INFINITY;

            for (idx, span) in spans.iter().enumerate().take(graph.layers().len()).skip(1) {
                let r_dist = if let Some(r_cut_idx) = r_cut {
                    (cut_states[r_cut_idx].index - idx as i32).abs()
                } else if let Some(l_cut_idx) = l_cut {
                    (idx as i32 - cut_states[l_cut_idx].index).abs() + 1
                } else {
                    1
                };

                let l_dist = if let Some(l_cut_idx) = l_cut {
                    (idx as i32 - cut_states[l_cut_idx].index).abs()
                } else {
                    r_dist + 1
                };

                let (hit, dist) = if l_dist < r_dist {
                    (l_cut, l_dist)
                } else {
                    (r_cut, r_dist)
                };

                let score = self.compute_score(graph, idx, *span, dist);
                if score < best_score {
                    best_score = score;
                    best_cut = hit;
                    if let Some(best_cut_idx) = best_cut {
                        cut_states[best_cut_idx].new_index = idx as i32;
                    }
                }

                if let Some(r_cut_idx) = r_cut {
                    if idx as i32 == cut_states[r_cut_idx].index {
                        l_cut = r_cut;
                        r_cut = cut_next(&cut_states, r_cut_idx);
                    }
                }
            }

            if let Some(best_cut_idx) = best_cut {
                improved.push(cut_states[best_cut_idx].new_index);
                cut_states[best_cut_idx].assigned = true;
                offset_assigned_cut(&mut cut_states, best_cut_idx);
            }
        }

        improved.sort();
        improved
    }

    fn compute_score(&self, graph: &mut LGraph, _index: usize, spans: i32, dist: i32) -> f64 {
        let distance_penalty = graph
            .get_property(LayeredOptions::WRAPPING_MULTI_EDGE_DISTANCE_PENALTY)
            .unwrap_or(2.0);
        spans as f64 + (dist as f64).powf(distance_penalty)
    }
}

#[derive(Clone)]
struct CutState {
    index: i32,
    new_index: i32,
    prev: Option<usize>,
    suc: Option<usize>,
    assigned: bool,
}

fn cut_self_or_next(cuts: &[CutState], cut: Option<usize>) -> Option<usize> {
    let mut current = cut;
    while let Some(idx) = current {
        if !cuts[idx].assigned {
            return Some(idx);
        }
        current = cuts[idx].suc;
    }
    None
}

fn cut_next(cuts: &[CutState], cut: usize) -> Option<usize> {
    cut_self_or_next(cuts, cuts[cut].suc)
}

fn offset_assigned_cut(cuts: &mut [CutState], cut: usize) {
    if !cuts[cut].assigned {
        return;
    }

    let offset = cuts[cut].new_index - cuts[cut].index;
    cuts[cut].index += offset;

    let mut prev = cuts[cut].prev;
    while let Some(prev_idx) = prev {
        if cuts[prev_idx].assigned {
            break;
        }
        cuts[prev_idx].index += offset;
        prev = cuts[prev_idx].prev;
    }

    let mut suc = cuts[cut].suc;
    while let Some(suc_idx) = suc {
        if cuts[suc_idx].assigned {
            break;
        }
        cuts[suc_idx].index += offset;
        suc = cuts[suc_idx].suc;
    }
}

fn compute_edge_spans(graph: &LGraph) -> Vec<i32> {
    let mut spans = vec![0; graph.layers().len() + 1];
    let mut open: Vec<LEdgeRef> = Vec::new();

    for (idx, layer) in graph.layers().iter().enumerate() {
        spans[idx] = open.len() as i32;

        let nodes = layer
            .lock().nodes().clone();

        for node in &nodes {
            let outgoing = node
                .lock().outgoing_edges();
            for edge in outgoing {
                if !contains_edge(&open, &edge) {
                    open.push(edge);
                }
            }
        }

        for node in &nodes {
            let incoming = node
                .lock().incoming_edges();
            for edge in incoming {
                remove_edge(&mut open, &edge);
            }
        }
    }

    spans
}

fn create_breaking_point_dummy(
    graph_ref: &LGraphRef,
    layer: &LayerRef,
) -> (
    LNodeRef,
    Arc<Mutex<crate::org::eclipse::elk::alg::layered::graph::LPort>>,
    Arc<Mutex<crate::org::eclipse::elk::alg::layered::graph::LPort>>,
) {
    let node = LNode::new(graph_ref);
    {
        let mut node_guard = node.lock();
        node_guard.set_property(
            LayeredOptions::PORT_CONSTRAINTS,
            Some(PortConstraints::FixedSide),
        );
        node_guard.set_node_type(NodeType::BreakingPoint);
    }
    LNode::set_layer(&node, Some(layer.clone()));

    let in_port = LPort::new();
    {
        let mut in_port_guard = in_port.lock();
        in_port_guard.set_side(PortSide::West);
    }
    LPort::set_node(&in_port, Some(node.clone()));

    let out_port = LPort::new();
    {
        let mut out_port_guard = out_port.lock();
        out_port_guard.set_side(PortSide::East);
    }
    LPort::set_node(&out_port, Some(node.clone()));

    (node, in_port, out_port)
}

fn contains_edge(edges: &[LEdgeRef], edge: &LEdgeRef) -> bool {
    edges.iter().any(|candidate| Arc::ptr_eq(candidate, edge))
}

fn remove_edge(edges: &mut Vec<LEdgeRef>, edge: &LEdgeRef) {
    if let Some(index) = edges
        .iter()
        .position(|candidate| Arc::ptr_eq(candidate, edge))
    {
        edges.remove(index);
    }
}

trait CutIndexCalculator {
    fn get_cut_indexes(&self, graph: &mut LGraph, graph_stats: &GraphStats) -> Vec<i32>;
    fn guarantee_valid(&self) -> bool;
}

struct ManualCutIndexCalculator;

impl CutIndexCalculator for ManualCutIndexCalculator {
    fn get_cut_indexes(&self, graph: &mut LGraph, _graph_stats: &GraphStats) -> Vec<i32> {
        graph
            .get_property(LayeredOptions::WRAPPING_CUTTING_CUTS)
            .unwrap_or_default()
    }

    fn guarantee_valid(&self) -> bool {
        false
    }
}

struct ARDCutIndexHeuristic;

impl ARDCutIndexHeuristic {
    fn get_chunk_count(graph_stats: &GraphStats) -> i32 {
        let denominator = graph_stats.dar * graph_stats.get_max_height();
        if denominator == 0.0 {
            return graph_stats.longest_path as i32;
        }
        let rows = (graph_stats.get_sum_width() / denominator).sqrt().round() as i32;
        rows.min(graph_stats.longest_path as i32)
    }
}

impl CutIndexCalculator for ARDCutIndexHeuristic {
    fn get_cut_indexes(&self, _graph: &mut LGraph, graph_stats: &GraphStats) -> Vec<i32> {
        let rows = Self::get_chunk_count(graph_stats);
        if rows <= 1 {
            return Vec::new();
        }

        let mut cuts = Vec::new();
        let step = graph_stats.longest_path as f64 / rows as f64;
        for idx in 1..rows {
            cuts.push((idx as f64 * step).round() as i32);
        }
        cuts
    }

    fn guarantee_valid(&self) -> bool {
        false
    }
}

struct MSDCutIndexHeuristic;

impl CutIndexCalculator for MSDCutIndexHeuristic {
    fn get_cut_indexes(&self, graph: &mut LGraph, graph_stats: &GraphStats) -> Vec<i32> {
        let widths = graph_stats.get_widths();
        let heights = graph_stats.get_heights();
        if widths.is_empty() {
            return Vec::new();
        }

        let mut width_at_index = vec![0.0; widths.len()];
        width_at_index[0] = widths[0];
        let mut total = widths[0];
        for i in 1..widths.len() {
            width_at_index[i] = width_at_index[i - 1] + widths[i];
            total += widths[i];
        }

        let cut_count = ARDCutIndexHeuristic::get_chunk_count(graph_stats) - 1;
        let freedom = graph
            .get_property(LayeredOptions::WRAPPING_CUTTING_MSD_FREEDOM)
            .unwrap_or(1);

        let mut best_max_scale = f64::NEG_INFINITY;
        let mut best_cuts = Vec::new();

        let min_m = (cut_count - freedom).max(0);
        let max_m = (cut_count + freedom).min(graph_stats.longest_path as i32 - 1);
        for m in min_m..=max_m {
            let row_sum = total / (m as f64 + 1.0);
            let mut sum_so_far = 0.0;
            let mut index = 1usize;
            let mut cuts = Vec::new();

            let mut width = f64::NEG_INFINITY;
            let mut last_cut_width = 0.0;
            let mut height = 0.0;
            let mut row_height_max = heights[0];

            if m == 0 {
                width = total;
                height = graph_stats.get_max_height();
            } else {
                while index < graph_stats.longest_path {
                    if width_at_index[index - 1] - sum_so_far >= row_sum {
                        cuts.push(index as i32);

                        width = width.max(width_at_index[index - 1] - last_cut_width);
                        height += row_height_max;

                        sum_so_far += width_at_index[index - 1] - sum_so_far;
                        last_cut_width = width_at_index[index - 1];
                        row_height_max = heights[index];
                    }

                    row_height_max = row_height_max.max(heights[index]);
                    index += 1;
                }

                height += row_height_max;
            }

            let width_scale = if width == 0.0 {
                f64::INFINITY
            } else {
                1.0 / width
            };
            let height_scale = if height == 0.0 {
                f64::INFINITY
            } else {
                (1.0 / graph_stats.dar) / height
            };
            let max_scale = width_scale.min(height_scale);

            if max_scale > best_max_scale {
                best_max_scale = max_scale;
                best_cuts = cuts;
            }
        }

        best_cuts
    }

    fn guarantee_valid(&self) -> bool {
        false
    }
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock().graph()
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock().graph() {
            return graph_ref;
        }
    }
    LGraph::new()
}
