use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector_chain::KVectorChain;
use org_eclipse_elk_core::org::eclipse::elk::core::options::core_options::CoreOptions;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_constraints::PortConstraints;
use org_eclipse_elk_core::org::eclipse::elk::core::options::port_side::PortSide;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{
    IElkProgressMonitor, IndividualSpacings,
};

use crate::org::eclipse::elk::alg::layered::graph::{
    LEdge, LEdgeRef, LGraph, LGraphRef, LNode, LNodeRef, LPort, LayerRef, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    CuttingStrategy, InternalProperties, LayeredOptions, Origin, ValidifyStrategy,
};

pub struct SingleEdgeGraphWrapper;

impl ILayoutProcessor<LGraph> for SingleEdgeGraphWrapper {
    fn process(&mut self, graph: &mut LGraph, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Path-Like Graph Wrapping", 1.0);

        if graph.layers().is_empty() {
            progress_monitor.done();
            return;
        }

        let graph_ref = graph_ref_for(graph);
        let graph_stats = GraphStats::new(graph);

        // Keep Java behavior: this simplifies to longestPath and is used as-is there.
        let sum_width = graph_stats.get_max_width() * graph_stats.longest_path as f64;
        let current_ar = if graph_stats.get_max_width() == 0.0 {
            0.0
        } else {
            sum_width / graph_stats.get_max_width()
        };
        if graph_stats.dar > current_ar {
            progress_monitor.done();
            return;
        }

        let cutting_strategy = graph
            .get_property(LayeredOptions::WRAPPING_CUTTING_STRATEGY)
            .unwrap_or(CuttingStrategy::Msd);

        let calculator: Box<dyn CutIndexCalculator> = match cutting_strategy {
            CuttingStrategy::Manual => Box::new(ManualCutIndexCalculator),
            CuttingStrategy::Ard => Box::new(ARDCutIndexHeuristic),
            CuttingStrategy::Msd => Box::new(MSDCutIndexHeuristic),
        };
        let mut cuts = calculator.get_cut_indexes(graph, &graph_stats);

        if !calculator.guarantee_valid() {
            let validify_strategy = graph
                .get_property(LayeredOptions::WRAPPING_VALIDIFY_STRATEGY)
                .unwrap_or(ValidifyStrategy::Greedy);
            cuts = match validify_strategy {
                ValidifyStrategy::LookBack => {
                    Self::validify_indexes_looking_back(&graph_stats, cuts)
                }
                ValidifyStrategy::Greedy => Self::validify_indexes_greedily(&graph_stats, cuts),
                ValidifyStrategy::No => cuts,
            };
        }

        self.perform_cuts(graph, &graph_ref, &graph_stats, &cuts);
        progress_monitor.done();
    }
}

impl SingleEdgeGraphWrapper {
    pub fn validify_indexes_greedily(graph_stats: &GraphStats, cuts: Vec<i32>) -> Vec<i32> {
        let mut valid_cuts = Vec::new();
        let mut offset = 0;

        for cut in cuts {
            let mut shifted = cut + offset;
            while shifted < graph_stats.longest_path as i32
                && !graph_stats.is_cut_allowed(shifted as usize)
            {
                shifted += 1;
                offset += 1;
            }
            if shifted >= graph_stats.longest_path as i32 {
                break;
            }
            valid_cuts.push(shifted);
        }

        valid_cuts
    }

    pub fn validify_indexes_looking_back(
        graph_stats: &GraphStats,
        desired_cuts: Vec<i32>,
    ) -> Vec<i32> {
        if desired_cuts.is_empty() {
            return Vec::new();
        }

        let mut valid_cuts = vec![i32::MIN];
        for idx in 1..graph_stats.longest_path {
            if graph_stats.is_cut_allowed(idx) {
                valid_cuts.push(idx as i32);
            }
        }
        if valid_cuts.len() == 1 {
            return Vec::new();
        }
        valid_cuts.push(i32::MAX);

        Self::validify_indexes_looking_back_with_valids(&desired_cuts, &valid_cuts)
    }

    fn validify_indexes_looking_back_with_valids(
        desired_cuts: &[i32],
        valid_cuts: &[i32],
    ) -> Vec<i32> {
        if desired_cuts.is_empty() || valid_cuts.len() < 2 {
            return Vec::new();
        }

        let mut final_cuts = Vec::new();
        let mut i_idx = 0usize;
        let mut c_idx = 0usize;
        let mut offset = 0i32;

        while i_idx < valid_cuts.len() - 1 && c_idx < desired_cuts.len() {
            let current = desired_cuts[c_idx] + offset;

            while i_idx + 1 < valid_cuts.len() && valid_cuts[i_idx + 1] < current {
                i_idx += 1;
            }
            if i_idx + 1 >= valid_cuts.len() {
                break;
            }

            let mut select = 0usize;
            let dist_lower = current.saturating_sub(valid_cuts[i_idx]);
            let dist_higher = valid_cuts[i_idx + 1].saturating_sub(current);
            if dist_lower > dist_higher {
                select = 1;
            }

            let selected = valid_cuts[i_idx + select];
            final_cuts.push(selected);
            offset += selected - current;

            c_idx += 1;
            while c_idx < desired_cuts.len() && desired_cuts[c_idx] + offset <= selected {
                c_idx += 1;
            }
            i_idx += 1 + select;
        }

        final_cuts
    }

    fn perform_cuts(
        &self,
        graph: &mut LGraph,
        graph_ref: &LGraphRef,
        graph_stats: &GraphStats,
        cuts: &[i32],
    ) {
        if cuts.is_empty() {
            return;
        }

        let mut index = 0usize;
        let mut new_index = 0usize;

        let mut cut_iter = cuts.iter();
        let mut next_cut = *cut_iter
            .next()
            .unwrap_or(&(graph_stats.longest_path as i32 + 1));

        while index < graph_stats.longest_path {
            if index as i32 == next_cut {
                new_index = 0;
                next_cut = cut_iter
                    .next()
                    .copied()
                    .unwrap_or(graph_stats.longest_path as i32 + 1);
            }

            if index != new_index {
                if index >= graph.layers().len() || new_index >= graph.layers().len() {
                    break;
                }
                let old_layer = graph.layers()[index].clone();
                let new_layer = graph.layers()[new_index].clone();

                let nodes_to_move = old_layer
                    .lock()
                    .ok()
                    .map(|layer_guard| layer_guard.nodes().clone())
                    .unwrap_or_default();
                for node in nodes_to_move {
                    let insert_index = new_layer
                        .lock()
                        .ok()
                        .map(|layer_guard| layer_guard.nodes().len())
                        .unwrap_or(0);
                    LNode::set_layer_at_index(&node, insert_index, Some(new_layer.clone()));

                    if new_index == 0 {
                        let incoming_edges = node
                            .lock()
                            .ok()
                            .map(|node_guard| node_guard.incoming_edges())
                            .unwrap_or_default();
                        for edge in incoming_edges {
                            LEdge::reverse(&edge, graph_ref, true);
                            graph.set_property(InternalProperties::CYCLIC, Some(true));
                            let _ = CuttingUtils::insert_dummies(graph, graph_ref, &edge, 1);
                        }
                    }
                }
            }

            new_index += 1;
            index += 1;
        }

        graph.layers_mut().retain(|layer| {
            !layer
                .lock()
                .ok()
                .map(|layer_guard| layer_guard.nodes().is_empty())
                .unwrap_or(true)
        });
    }
}

pub struct GraphStats {
    pub dar: f64,
    pub longest_path: usize,
    max_width: f64,
    max_height: f64,
    sum_width: f64,
    widths: Vec<f64>,
    heights: Vec<f64>,
    cuts_allowed: Vec<bool>,
}

impl GraphStats {
    pub fn new(graph: &mut LGraph) -> Self {
        let direction = graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        let aspect_ratio = graph
            .get_property(LayeredOptions::ASPECT_RATIO)
            .unwrap_or(1.0);
        let correction = graph
            .get_property(LayeredOptions::WRAPPING_CORRECTION_FACTOR)
            .unwrap_or(1.0);
        let dar = if matches!(
            direction,
            Direction::Left | Direction::Right | Direction::Undefined
        ) {
            aspect_ratio * correction
        } else {
            1.0 / (aspect_ratio * correction)
        };

        let spacing = graph
            .get_property(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        let in_layer_spacing = graph
            .get_property(LayeredOptions::SPACING_NODE_NODE)
            .unwrap_or(0.0);

        let layers = graph.layers().clone();
        let longest_path = layers.len();

        let mut widths = Vec::with_capacity(longest_path);
        let mut heights = Vec::with_capacity(longest_path);
        for layer in &layers {
            widths.push(determine_layer_width(layer, spacing));
            heights.push(determine_layer_height(layer, in_layer_spacing));
        }

        let max_width = widths.iter().copied().reduce(f64::max).unwrap_or(0.0);
        let max_height = heights.iter().copied().reduce(f64::max).unwrap_or(0.0);
        let sum_width = widths.iter().sum();
        let cuts_allowed = init_cut_allowed(graph, &layers);

        Self {
            dar,
            longest_path,
            max_width,
            max_height,
            sum_width,
            widths,
            heights,
            cuts_allowed,
        }
    }

    pub fn get_max_width(&self) -> f64 {
        self.max_width
    }

    pub fn get_sum_width(&self) -> f64 {
        self.sum_width
    }

    pub fn get_widths(&self) -> Vec<f64> {
        self.widths.clone()
    }

    pub fn get_max_height(&self) -> f64 {
        self.max_height
    }

    pub fn get_heights(&self) -> Vec<f64> {
        self.heights.clone()
    }

    pub fn is_cut_allowed(&self, layer_index: usize) -> bool {
        self.cuts_allowed.get(layer_index).copied().unwrap_or(false)
    }
}

fn determine_layer_width(layer: &LayerRef, spacing: f64) -> f64 {
    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    let mut max_width: f64 = 0.0;
    for node in nodes {
        if let Ok(mut node_guard) = node.lock() {
            let node_width = node_guard.shape().size_ref().x
                + node_guard.margin().left
                + node_guard.margin().right
                + spacing;
            max_width = max_width.max(node_width);
        }
    }
    max_width
}

fn determine_layer_height(layer: &LayerRef, in_layer_spacing: f64) -> f64 {
    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    let mut layer_height = 0.0;
    for node in nodes {
        let incoming_edges = node
            .lock()
            .ok()
            .map(|node_guard| node_guard.incoming_edges())
            .unwrap_or_default();

        if let Ok(mut node_guard) = node.lock() {
            layer_height += node_guard.shape().size_ref().y
                + node_guard.margin().bottom
                + node_guard.margin().top
                + in_layer_spacing;
        }

        for incoming_edge in incoming_edges {
            let source_node = incoming_edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            let Some(source_node) = source_node else {
                continue;
            };

            let is_north_south_dummy = source_node
                .lock()
                .ok()
                .map(|node_guard| node_guard.node_type() == NodeType::NorthSouthPort)
                .unwrap_or(false);
            if !is_north_south_dummy {
                continue;
            }

            let origin = source_node
                .lock()
                .ok()
                .and_then(|mut node_guard| node_guard.get_property(InternalProperties::ORIGIN));
            if let Some(Origin::LNode(origin_node)) = origin {
                if let Ok(mut origin_guard) = origin_node.lock() {
                    layer_height += origin_guard.shape().size_ref().y
                        + origin_guard.margin().bottom
                        + origin_guard.margin().top;
                }
            }
        }
    }

    layer_height
}

fn init_cut_allowed(graph: &mut LGraph, layers: &[LayerRef]) -> Vec<bool> {
    let mut cuts_allowed = vec![false; layers.len()];
    if !cuts_allowed.is_empty() {
        cuts_allowed[0] = false;
    }

    if graph
        .graph_element()
        .properties()
        .has_property(LayeredOptions::WRAPPING_VALIDIFY_FORBIDDEN_INDICES)
    {
        let forbidden = graph
            .get_property(LayeredOptions::WRAPPING_VALIDIFY_FORBIDDEN_INDICES)
            .unwrap_or_default();
        for forbidden_index in forbidden {
            if forbidden_index > 0 && (forbidden_index as usize) < cuts_allowed.len() {
                cuts_allowed[forbidden_index as usize] = false;
            }
        }
        return cuts_allowed;
    }

    for (idx, layer) in layers.iter().enumerate().skip(1) {
        cuts_allowed[idx] = is_cut_allowed_layer(layer);
    }

    cuts_allowed
}

fn is_cut_allowed_layer(layer: &LayerRef) -> bool {
    let nodes = layer
        .lock()
        .ok()
        .map(|layer_guard| layer_guard.nodes().clone())
        .unwrap_or_default();

    let mut target_node: Option<LNodeRef> = None;
    let mut source_node: Option<LNodeRef> = None;
    for target in nodes {
        let incoming_edges = target
            .lock()
            .ok()
            .map(|node_guard| node_guard.incoming_edges())
            .unwrap_or_default();
        for edge in incoming_edges {
            if let Some(existing_target) = &target_node {
                if !Arc::ptr_eq(existing_target, &target) {
                    return false;
                }
            } else {
                target_node = Some(target.clone());
            }

            let source = edge
                .lock()
                .ok()
                .and_then(|edge_guard| edge_guard.source())
                .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
            let Some(source) = source else {
                continue;
            };

            if let Some(existing_source) = &source_node {
                if !Arc::ptr_eq(existing_source, &source) {
                    return false;
                }
            } else {
                source_node = Some(source);
            }
        }
    }

    true
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

pub(crate) struct CuttingUtils;

impl CuttingUtils {
    pub fn insert_dummies(
        layered_graph: &mut LGraph,
        graph_ref: &LGraphRef,
        original_edge: &LEdgeRef,
        offset_first_in_layer_dummy: usize,
    ) -> Vec<LEdgeRef> {
        let edge_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_NODE)
            .unwrap_or(0.0);
        let additional_spacing = layered_graph
            .get_property(LayeredOptions::WRAPPING_ADDITIONAL_EDGE_SPACING)
            .unwrap_or(0.0);

        let mut individual_spacings = IndividualSpacings::new();
        individual_spacings.properties_mut().set_property(
            LayeredOptions::SPACING_EDGE_NODE,
            Some(edge_node_spacing + additional_spacing),
        );

        let mut edge = original_edge.clone();
        let target_port = edge.lock().ok().and_then(|edge_guard| edge_guard.target());

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
            return Vec::new();
        };

        let source_layer = source_node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.layer());
        let target_layer = target_node
            .lock()
            .ok()
            .and_then(|node_guard| node_guard.layer());
        let (Some(source_layer), Some(target_layer)) = (source_layer, target_layer) else {
            return Vec::new();
        };

        let src_index = layered_graph.index_of_layer(&source_layer).unwrap_or(0);
        let tgt_index = layered_graph
            .index_of_layer(&target_layer)
            .unwrap_or(src_index);

        let mut created_edges = Vec::new();
        for layer_index in src_index..=tgt_index {
            let dummy_node = LNode::new(graph_ref);
            if let Ok(mut dummy_guard) = dummy_node.lock() {
                dummy_guard.set_node_type(NodeType::LongEdge);
                dummy_guard.set_property(
                    InternalProperties::ORIGIN,
                    Some(Origin::LEdge(edge.clone())),
                );
                dummy_guard.set_property(
                    LayeredOptions::PORT_CONSTRAINTS,
                    Some(PortConstraints::FixedPos),
                );
                dummy_guard.set_property(
                    LayeredOptions::SPACING_INDIVIDUAL,
                    Some(individual_spacings.clone()),
                );
            }

            let Some(next_layer) = layered_graph.layers().get(layer_index).cloned() else {
                break;
            };
            if layer_index == src_index {
                let insertion_index = next_layer
                    .lock()
                    .ok()
                    .map(|layer_guard| {
                        if layer_guard.nodes().len() > offset_first_in_layer_dummy {
                            layer_guard.nodes().len() - offset_first_in_layer_dummy
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                LNode::set_layer_at_index(&dummy_node, insertion_index, Some(next_layer.clone()));
            } else {
                LNode::set_layer(&dummy_node, Some(next_layer.clone()));
            }

            let mut thickness = edge
                .lock()
                .ok()
                .and_then(|mut edge_guard| edge_guard.get_property(CoreOptions::EDGE_THICKNESS))
                .unwrap_or(1.0);
            if thickness < 0.0 {
                thickness = 0.0;
                if let Ok(mut edge_guard) = edge.lock() {
                    edge_guard.set_property(CoreOptions::EDGE_THICKNESS, Some(thickness));
                }
            }

            if let Ok(mut dummy_guard) = dummy_node.lock() {
                dummy_guard.shape().size().y = thickness;
            }
            let port_pos = (thickness / 2.0).floor();

            let dummy_input = LPort::new();
            if let Ok(mut input_guard) = dummy_input.lock() {
                input_guard.set_side(PortSide::West);
                input_guard.shape().position().y = port_pos;
            }
            LPort::set_node(&dummy_input, Some(dummy_node.clone()));

            let dummy_output = LPort::new();
            if let Ok(mut output_guard) = dummy_output.lock() {
                output_guard.set_side(PortSide::East);
                output_guard.shape().position().y = port_pos;
            }
            LPort::set_node(&dummy_output, Some(dummy_node.clone()));

            LEdge::set_target(&edge, Some(dummy_input));

            let dummy_edge = LEdge::new();
            if let (Ok(mut new_edge_guard), Ok(mut old_edge_guard)) =
                (dummy_edge.lock(), edge.lock())
            {
                new_edge_guard
                    .graph_element()
                    .properties_mut()
                    .copy_properties(old_edge_guard.graph_element().properties());
                new_edge_guard.set_property(LayeredOptions::JUNCTION_POINTS, None::<KVectorChain>);
            }
            LEdge::set_source(&dummy_edge, Some(dummy_output));
            if let Some(target_port) = target_port.clone() {
                LEdge::set_target(&dummy_edge, Some(target_port));
            }

            Self::set_dummy_properties(&dummy_node, &edge, &dummy_edge);
            created_edges.push(dummy_edge.clone());
            edge = dummy_edge;
        }

        created_edges
    }

    fn set_dummy_properties(dummy: &LNodeRef, in_edge: &LEdgeRef, out_edge: &LEdgeRef) {
        let in_edge_source = in_edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.source());
        let out_edge_target = out_edge
            .lock()
            .ok()
            .and_then(|edge_guard| edge_guard.target());

        let in_edge_source_node = in_edge_source
            .as_ref()
            .and_then(|port| port.lock().ok().and_then(|port_guard| port_guard.node()));
        let in_edge_source_type = in_edge_source_node
            .as_ref()
            .and_then(|node| node.lock().ok().map(|node_guard| node_guard.node_type()));

        if in_edge_source_type == Some(NodeType::LongEdge) {
            if let Some(in_edge_source_node) = in_edge_source_node {
                if let (Ok(mut dummy_guard), Ok(mut source_guard)) =
                    (dummy.lock(), in_edge_source_node.lock())
                {
                    dummy_guard.set_property(
                        InternalProperties::LONG_EDGE_SOURCE,
                        source_guard.get_property(InternalProperties::LONG_EDGE_SOURCE),
                    );
                    dummy_guard.set_property(
                        InternalProperties::LONG_EDGE_TARGET,
                        source_guard.get_property(InternalProperties::LONG_EDGE_TARGET),
                    );
                }
            }
            return;
        }

        if let Ok(mut dummy_guard) = dummy.lock() {
            dummy_guard.set_property(InternalProperties::LONG_EDGE_SOURCE, in_edge_source);
            dummy_guard.set_property(InternalProperties::LONG_EDGE_TARGET, out_edge_target);
        }
    }
}

fn graph_ref_for(layered_graph: &LGraph) -> LGraphRef {
    if let Some(layer) = layered_graph.layers().first() {
        if let Some(graph_ref) = layer
            .lock()
            .ok()
            .and_then(|layer_guard| layer_guard.graph())
        {
            return graph_ref;
        }
    }
    if let Some(node) = layered_graph.layerless_nodes().first() {
        if let Some(graph_ref) = node.lock().ok().and_then(|node_guard| node_guard.graph()) {
            return graph_ref;
        }
    }
    LGraph::new()
}
