use std::collections::VecDeque;
use std::sync::{Arc, LazyLock};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_phase::ILayoutPhase;
use org_eclipse_elk_core::org::eclipse::elk::core::alg::layout_processor_configuration::LayoutProcessorConfiguration;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::{ArenaSync, LGraph, LNodeRef, LPortRef, NodeType};
use crate::org::eclipse::elk::alg::layered::intermediate::IntermediateProcessorStrategy;
use crate::org::eclipse::elk::alg::layered::options::{
    GraphProperties, InternalProperties, LayeredOptions, Spacings,
};
use crate::org::eclipse::elk::alg::layered::LayeredPhases;

static HIERARCHY_PROCESSING_ADDITIONS: LazyLock<
    LayoutProcessorConfiguration<LayeredPhases, LGraph>,
> = LazyLock::new(|| {
    let mut config = LayoutProcessorConfiguration::create();
    config.add_before(
        LayeredPhases::P5EdgeRouting,
        Arc::new(IntermediateProcessorStrategy::HierarchicalPortPositionProcessor),
    );
    config
});

static INPUT_PRIO_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("linearSegments.inputPrio"));
static OUTPUT_PRIO_PROPERTY: LazyLock<Property<i32>> =
    LazyLock::new(|| Property::new("linearSegments.outputPrio"));

#[derive(Clone)]
struct LinearSegment {
    nodes: Vec<LNodeRef>,
    id: usize,
    index_in_last_layer: isize,
    last_layer: isize,
    deflection: f64,
    weight: i32,
    ref_segment: Option<usize>,
    node_type: NodeType,
}

impl LinearSegment {
    fn new(id: usize) -> Self {
        LinearSegment {
            nodes: Vec::new(),
            id,
            index_in_last_layer: -1,
            last_layer: -1,
            deflection: 0.0,
            weight: 0,
            ref_segment: None,
            node_type: NodeType::Normal,
        }
    }

    fn split(&mut self, node: &LNodeRef, new_id: usize) -> LinearSegment {
        let node_index = self
            .nodes
            .iter()
            .position(|candidate| Arc::ptr_eq(candidate, node))
            .unwrap_or(0);
        let new_nodes = self.nodes.split_off(node_index);
        for moved in &new_nodes {
            set_node_id(moved, new_id as i32);
        }
        let mut new_segment = LinearSegment::new(new_id);
        new_segment.nodes = new_nodes;
        new_segment.node_type = self.node_type;
        new_segment
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    ForwPendulum,
    BackwPendulum,
    Rubber,
}

pub struct LinearSegmentsNodePlacer {
    linear_segments: Vec<LinearSegment>,
    spacings: Option<Spacings>,
    sync: Option<ArenaSync>,
}

impl LinearSegmentsNodePlacer {
    pub fn new() -> Self {
        LinearSegmentsNodePlacer {
            linear_segments: Vec::new(),
            spacings: None,
            sync: None,
        }
    }

    fn sort_linear_segments(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        let layers = graph.layers().clone();

        // initialize node ids and priorities
        for layer_ref in layers.iter() {
            let nodes = layer_ref
                .lock().nodes().clone();
            for node in nodes {
                set_node_id(&node, -1);
                let mut inprio = i32::MIN;
                let mut outprio = i32::MIN;
                let ports = node
                    .lock().ports().clone();
                for port in ports {
                    let incoming = port
                        .lock().incoming_edges().clone();
                    for edge in incoming {
                        let prio = {
                            let edge_guard = edge.lock();
                            edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
                                .unwrap_or(0)
                        };
                        inprio = inprio.max(prio);
                    }
                    let outgoing = port
                        .lock().outgoing_edges().clone();
                    for edge in outgoing {
                        let prio = {
                            let edge_guard = edge.lock();
                            edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
                                .unwrap_or(0)
                        };
                        outprio = outprio.max(prio);
                    }
                }
                {
                    let mut node_guard = node.lock();
                    node_guard.set_property(&INPUT_PRIO_PROPERTY, Some(inprio));
                    node_guard.set_property(&OUTPUT_PRIO_PROPERTY, Some(outprio));
                }
            }
        }

        let mut segment_list: Vec<LinearSegment> = Vec::new();
        let mut next_id: usize = 0;
        for layer_ref in layers.iter() {
            let nodes = layer_ref
                .lock().nodes().clone();
            for node in nodes {
                if node_id(&node) < 0 {
                    let mut segment = LinearSegment::new(next_id);
                    next_id += 1;
                    Self::fill_segment(&node, &mut segment);
                    segment_list.push(segment);
                }
            }
        }

        let mut outgoing_list: Vec<Vec<usize>> = vec![Vec::new(); segment_list.len()];
        let mut incoming_count: Vec<i32> = vec![0; segment_list.len()];
        self.create_dependency_graph_edges(
            monitor,
            graph,
            &mut segment_list,
            &mut outgoing_list,
            &mut incoming_count,
        );

        let mut no_incoming: VecDeque<usize> = VecDeque::new();
        for (idx, count) in incoming_count.iter().enumerate() {
            if *count == 0 {
                no_incoming.push_back(idx);
            }
        }

        let mut new_ranks = vec![0usize; segment_list.len()];
        let mut next_rank = 0usize;
        while let Some(seg_index) = no_incoming.pop_front() {
            let seg_id = segment_list[seg_index].id;
            new_ranks[seg_id] = next_rank;
            next_rank += 1;

            let mut targets = Vec::new();
            std::mem::swap(&mut targets, &mut outgoing_list[seg_id]);
            for target in targets {
                if incoming_count[target] > 0 {
                    incoming_count[target] -= 1;
                    if incoming_count[target] == 0 {
                        no_incoming.push_back(target);
                    }
                }
            }
        }

        let mut linear_segments = vec![LinearSegment::new(0); segment_list.len()];
        for segment in segment_list.into_iter() {
            let rank = new_ranks[segment.id];
            let mut moved = segment.clone();
            moved.id = rank;
            for node in moved.nodes.iter() {
                set_node_id(node, rank as i32);
            }
            linear_segments[rank] = moved;
        }
        self.linear_segments = linear_segments;
    }

    fn create_dependency_graph_edges(
        &mut self,
        monitor: &mut dyn IElkProgressMonitor,
        graph: &mut LGraph,
        segment_list: &mut Vec<LinearSegment>,
        outgoing_list: &mut Vec<Vec<usize>>,
        incoming_count: &mut Vec<i32>,
    ) {
        let layers = graph.layers().clone();
        let mut next_id = segment_list.len();
        let mut layer_index = 0usize;

        for layer_ref in layers {
            let nodes = layer_ref
                .lock().nodes().clone();
            if nodes.is_empty() {
                continue;
            }

            let mut index_in_layer = 0usize;
            let mut previous_node: Option<LNodeRef> = None;

            let mut node_iter = nodes.iter();
            let mut current_node = node_iter.next().cloned();

            while let Some(node) = current_node {
                let current_segment_id = node_id(&node).max(0) as usize;

                let cycle_segment_id = {
                    let seg = &segment_list[current_segment_id];
                    if seg.index_in_last_layer >= 0 {
                        let mut found = None;
                        for cycle_node in nodes.iter().skip(index_in_layer + 1) {
                            let cycle_id = node_id(cycle_node).max(0) as usize;
                            let cycle_seg = &segment_list[cycle_id];
                            if cycle_seg.last_layer == seg.last_layer
                                && cycle_seg.index_in_last_layer < seg.index_in_last_layer
                            {
                                found = Some(cycle_id);
                                break;
                            }
                        }
                        found
                    } else {
                        None
                    }
                };

                let mut current_segment_id = current_segment_id;
                if cycle_segment_id.is_some() {
                    if let Some(prev_node) = previous_node.as_ref() {
                        let prev_id = node_id(prev_node).max(0) as usize;
                        if incoming_count[current_segment_id] > 0 {
                            incoming_count[current_segment_id] -= 1;
                        }
                        outgoing_list[prev_id].retain(|id| *id != current_segment_id);
                    }

                    let new_segment = {
                        let segment = &mut segment_list[current_segment_id];
                        segment.split(&node, next_id)
                    };
                    segment_list.push(new_segment);
                    outgoing_list.push(Vec::new());
                    incoming_count.push(if previous_node.is_some() { 1 } else { 0 });
                    if let Some(prev_node) = previous_node.as_ref() {
                        let prev_id = node_id(prev_node).max(0) as usize;
                        outgoing_list[prev_id].push(next_id);
                    }
                    current_segment_id = next_id;
                    next_id += 1;
                }

                if let Some(next_node) = node_iter.next().cloned() {
                    let next_segment_id = node_id(&next_node).max(0) as usize;
                    outgoing_list[current_segment_id].push(next_segment_id);
                    incoming_count[next_segment_id] += 1;
                    current_node = Some(next_node);
                } else {
                    current_node = None;
                }

                {
                    let segment = &mut segment_list[current_segment_id];
                    segment.last_layer = layer_index as isize;
                    segment.index_in_last_layer = index_in_layer as isize;
                }

                previous_node = Some(node);
                index_in_layer += 1;
            }

            layer_index += 1;
        }

        let _ = monitor;
    }

    fn fill_segment(node: &LNodeRef, segment: &mut LinearSegment) -> bool {
        if node_id(node) >= 0 {
            return false;
        }

        set_node_id(node, segment.id as i32);
        segment.nodes.push(node.clone());

        let node_type = node
            .lock().node_type();
        segment.node_type = node_type;

        if matches!(node_type, NodeType::LongEdge | NodeType::NorthSouthPort) {
            let ports = node
                .lock().ports().clone();
            for source_port in ports {
                let successors = source_port
                    .lock().successor_ports();
                for target_port in successors {
                    let target_node = target_port
                        .lock().node();
                    let Some(target_node) = target_node else {
                        continue;
                    };
                    let target_type = target_node
                        .lock().node_type();
                    if layer_index(node) != layer_index(&target_node)
                        && matches!(target_type, NodeType::LongEdge | NodeType::NorthSouthPort)
                        && Self::fill_segment(&target_node, segment)
                    {
                        return true;
                    }
                }
            }
        }

        true
    }

    fn create_unbalanced_placement(&mut self, graph: &mut LGraph) {
        let spacings = self
            .spacings
            .as_ref()
            .expect("spacings required for unbalanced placement");
        let layers = graph.layers().clone();
        let layer_count = layers.len();
        let mut node_count = vec![0usize; layer_count];
        let mut recent_node: Vec<Option<LNodeRef>> = vec![None; layer_count];

        for segment in &self.linear_segments {
            let mut uppermost_place: f64 = 0.0;
            for node in &segment.nodes {
                let layer_idx = layer_index(node);
                if layer_idx >= layer_count {
                    continue;
                }
                node_count[layer_idx] = node_count[layer_idx].saturating_add(1);
                let spacing = match recent_node[layer_idx].as_ref() {
                    Some(prev) => spacings.get_vertical_spacing(prev, node),
                    None => graph
                        .get_property(LayeredOptions::SPACING_EDGE_EDGE)
                        .unwrap_or(0.0),
                };
                let layer_size = {
                    let layer_guard = layers[layer_idx].lock();
                    layer_guard.size_ref().y
                };
                uppermost_place = uppermost_place.max(layer_size + spacing);
            }

            for node in &segment.nodes {
                let layer_idx = layer_index(node);
                let s = self.sync.as_ref().unwrap();
                let margin_top = node_margin_top_a(s, node);
                let margin_bottom = node_margin_bottom_a(s, node);
                let size_y = node_size_y_a(s, node);
                {
                    let mut node_guard = node.lock();
                    node_guard.shape().position().y = uppermost_place + margin_top;
                }
                if let Some(layer_ref) = layers.get(layer_idx) {
                    let mut layer_guard = layer_ref.lock();
                    layer_guard.size().y =
                        uppermost_place + margin_top + size_y + margin_bottom;
                }
                if layer_idx < recent_node.len() {
                    recent_node[layer_idx] = Some(node.clone());
                }
            }
        }
    }

    fn balance_placement(&mut self, graph: &mut LGraph) {
        let spacings = self
            .spacings
            .as_ref()
            .expect("spacings required for balancing")
            .clone();
        let deflection_dampening = graph
            .get_property(LayeredOptions::NODE_PLACEMENT_LINEAR_SEGMENTS_DEFLECTION_DAMPENING)
            .unwrap_or(0.0);
        let thoroughness = graph
            .get_property(LayeredOptions::THOROUGHNESS)
            .unwrap_or(1);

        let mut pendulum_iters = 4;
        let mut final_iters = 3;
        let threshold = 20.0 / thoroughness as f64;
        let mut ready = false;
        let mut mode = Mode::ForwPendulum;
        let mut last_total_deflection = f64::MAX;

        loop {
            let incoming = mode != Mode::BackwPendulum;
            let outgoing = mode != Mode::ForwPendulum;
            let mut total_deflection = 0.0;
            for index in 0..self.linear_segments.len() {
                self.linear_segments[index].ref_segment = None;
                let (deflection, weight) =
                    self.compute_deflection(index, incoming, outgoing, deflection_dampening);
                self.linear_segments[index].deflection = deflection;
                self.linear_segments[index].weight = weight;
                total_deflection += deflection.abs();
            }

            while self.merge_regions(graph, &spacings) {}

            for index in 0..self.linear_segments.len() {
                let region_idx = region_index(&self.linear_segments, index);
                let deflection = self.linear_segments[region_idx].deflection;
                if deflection != 0.0 {
                    for node in self.linear_segments[index].nodes.iter() {
                        {
                            let mut node_guard = node.lock();
                            node_guard.shape().position().y += deflection;
                        }
                    }
                }
            }

            match mode {
                Mode::ForwPendulum | Mode::BackwPendulum => {
                    pendulum_iters -= 1;
                    if pendulum_iters <= 0
                        && (total_deflection < last_total_deflection
                            || -pendulum_iters > thoroughness)
                    {
                        mode = Mode::Rubber;
                        last_total_deflection = f64::MAX;
                    } else if mode == Mode::ForwPendulum {
                        mode = Mode::BackwPendulum;
                        last_total_deflection = total_deflection;
                    } else {
                        mode = Mode::ForwPendulum;
                        last_total_deflection = total_deflection;
                    }
                }
                Mode::Rubber => {
                    ready = total_deflection >= last_total_deflection
                        || last_total_deflection - total_deflection < threshold;
                    last_total_deflection = total_deflection;
                    if ready {
                        final_iters -= 1;
                    }
                }
            }

            if ready && final_iters <= 0 {
                break;
            }
        }
    }

    fn compute_deflection(
        &self,
        segment_index: usize,
        incoming: bool,
        outgoing: bool,
        deflection_dampening: f64,
    ) -> (f64, i32) {
        let mut segment_deflection = 0.0;
        let mut node_weight_sum = 0;
        let segment = &self.linear_segments[segment_index];

        for node in &segment.nodes {
            let mut node_deflection = 0.0;
            let mut edge_weight_sum = 0;
            let (input_prio, output_prio) = {
                let guard = node.lock();                let input_prio = if incoming {
                    guard.get_property(&INPUT_PRIO_PROPERTY).unwrap_or(i32::MIN)
                } else {
                    i32::MIN
                };
                let output_prio = if outgoing {
                    guard
                        .get_property(&OUTPUT_PRIO_PROPERTY)
                        .unwrap_or(i32::MIN)
                } else {
                    i32::MIN
                };
                (input_prio, output_prio)
            };
            let min_prio = input_prio.max(output_prio);

            let ports = node
                .lock().ports().clone();
            let s = self.sync.as_ref().unwrap();
            for port in ports {
                let port_pos = port_position_y_a(s, node, &port);
                if outgoing {
                    let outgoing_edges = port
                        .lock().outgoing_edges().clone();
                    for edge in outgoing_edges {
                        let other_port =
                            edge.lock().target();
                        let Some(other_port) = other_port else {
                            continue;
                        };
                        let other_node = other_port
                            .lock().node();
                        let Some(other_node) = other_node else {
                            continue;
                        };
                        let other_segment = node_id(&other_node).max(0) as usize;
                        if other_segment != segment_index {
                            let other_prio = {
                                let node_guard = other_node.lock();
                                let input = node_guard
                                    .get_property(&INPUT_PRIO_PROPERTY)
                                    .unwrap_or(i32::MIN);
                                let output = node_guard
                                    .get_property(&OUTPUT_PRIO_PROPERTY)
                                    .unwrap_or(i32::MIN);
                                input.max(output)
                            };
                            let prio = {
                                let edge_guard = edge.lock();
                                edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
                                    .unwrap_or(0)
                            };
                            if prio >= min_prio && prio >= other_prio {
                                node_deflection +=
                                    port_position_y_a(s, &other_node, &other_port) - port_pos;
                                edge_weight_sum += 1;
                            }
                        }
                    }
                }

                if incoming {
                    let incoming_edges = port
                        .lock().incoming_edges().clone();
                    for edge in incoming_edges {
                        let other_port =
                            edge.lock().source();
                        let Some(other_port) = other_port else {
                            continue;
                        };
                        let other_node = other_port
                            .lock().node();
                        let Some(other_node) = other_node else {
                            continue;
                        };
                        let other_segment = node_id(&other_node).max(0) as usize;
                        if other_segment != segment_index {
                            let other_prio = {
                                let node_guard = other_node.lock();
                                let input = node_guard
                                    .get_property(&INPUT_PRIO_PROPERTY)
                                    .unwrap_or(i32::MIN);
                                let output = node_guard
                                    .get_property(&OUTPUT_PRIO_PROPERTY)
                                    .unwrap_or(i32::MIN);
                                input.max(output)
                            };
                            let prio = {
                                let edge_guard = edge.lock();
                                edge_guard.get_property(LayeredOptions::PRIORITY_STRAIGHTNESS)
                                    .unwrap_or(0)
                            };
                            if prio >= min_prio && prio >= other_prio {
                                node_deflection +=
                                    port_position_y_a(s, &other_node, &other_port) - port_pos;
                                edge_weight_sum += 1;
                            }
                        }
                    }
                }
            }

            if edge_weight_sum > 0 {
                segment_deflection += node_deflection / edge_weight_sum as f64;
                node_weight_sum += 1;
            }
        }

        if node_weight_sum > 0 {
            (
                deflection_dampening * segment_deflection / node_weight_sum as f64,
                node_weight_sum,
            )
        } else {
            (0.0, 0)
        }
    }

    fn merge_regions(&mut self, graph: &mut LGraph, spacings: &Spacings) -> bool {
        let node_spacing = graph
            .get_property(LayeredOptions::SPACING_NODE_NODE)
            .unwrap_or(0.0);
        let threshold = 0.0001 * node_spacing;

        let layers = graph.layers().clone();
        let mut changed = false;
        for layer_ref in layers {
            let nodes = layer_ref
                .lock().nodes().clone();
            if nodes.is_empty() {
                continue;
            }
            let mut iter = nodes.into_iter();
            let mut node1 = iter.next().unwrap();
            let mut region1 = region_index(&self.linear_segments, node_id(&node1).max(0) as usize);
            for node2 in iter {
                let region2 = region_index(&self.linear_segments, node_id(&node2).max(0) as usize);
                if region1 != region2 {
                    let spacing = spacings.get_vertical_spacing(&node1, &node2);
                    let s = self.sync.as_ref().unwrap();
                    let node1_extent = node_position_y(&node1)
                        + node_size_y_a(s, &node1)
                        + node_margin_bottom_a(s, &node1)
                        + self.linear_segments[region1].deflection
                        + spacing;
                    let node2_extent = node_position_y(&node2) - node_margin_top_a(s, &node2)
                        + self.linear_segments[region2].deflection;
                    if node1_extent > node2_extent + threshold {
                        let weight_sum = self.linear_segments[region1].weight
                            + self.linear_segments[region2].weight;
                        if weight_sum > 0 {
                            let new_deflection = (self.linear_segments[region2].weight as f64
                                * self.linear_segments[region2].deflection
                                + self.linear_segments[region1].weight as f64
                                    * self.linear_segments[region1].deflection)
                                / weight_sum as f64;
                            self.linear_segments[region2].deflection = new_deflection;
                            self.linear_segments[region2].weight = weight_sum;
                            self.linear_segments[region1].ref_segment = Some(region2);
                            changed = true;
                        }
                    }
                }
                node1 = node2;
                region1 = region2;
            }
        }

        changed
    }

    fn post_process(&mut self, graph: &mut LGraph) {
        let spacings = self
            .spacings
            .as_ref()
            .expect("spacings required for post processing");
        let _ = graph;

        for segment in &self.linear_segments {
            let mut min_room_above = f64::MAX;
            let mut min_room_below = f64::MAX;

            for node in &segment.nodes {
                let index = node_index(node);
                let layer_nodes = node
                    .lock().layer()
                    .map(|layer| {
                        let layer_guard = layer.lock();
                        layer_guard.nodes().clone()
                    })
                    .unwrap_or_default();

                let s = self.sync.as_ref().unwrap();
                let room_above = if index > 0 && index <= layer_nodes.len() {
                    let neighbor = &layer_nodes[index - 1];
                    let spacing = spacings.get_vertical_spacing(node, neighbor);
                    node_position_y(node)
                        - node_margin_top_a(s, node)
                        - (node_position_y(neighbor)
                            + node_size_y_a(s, neighbor)
                            + node_margin_bottom_a(s, neighbor)
                            + spacing)
                } else {
                    node_position_y(node) - node_margin_top_a(s, node)
                };
                min_room_above = min_room_above.min(room_above);

                let room_below = if index + 1 < layer_nodes.len() {
                    let neighbor = &layer_nodes[index + 1];
                    let spacing = spacings.get_vertical_spacing(node, neighbor);
                    node_position_y(neighbor)
                        - node_margin_top_a(s, neighbor)
                        - (node_position_y(node)
                            + node_size_y_a(s, node)
                            + node_margin_bottom_a(s, node)
                            + spacing)
                } else {
                    2.0 * node_position_y(node)
                };
                min_room_below = min_room_below.min(room_below);
            }

            let mut min_displacement = f64::MAX;
            let mut found_place = false;

            if let Some(first_node) = segment.nodes.first() {
                let s = self.sync.as_ref().unwrap();
                let ports = first_node
                    .lock().ports().clone();
                for target in ports {
                    let pos = port_position_y_a(s, first_node, &target);
                    let incoming = target
                        .lock().incoming_edges().clone();
                    for edge in incoming {
                        let source = edge.lock().source();
                        let Some(source) = source else {
                            continue;
                        };
                        let other_node =
                            source.lock().node();
                        let Some(other_node) = other_node else {
                            continue;
                        };
                        let d = port_position_y_a(s, &other_node, &source) - pos;
                        if d.abs() < min_displacement.abs()
                            && d.abs()
                                < if d < 0.0 {
                                    min_room_above
                                } else {
                                    min_room_below
                                }
                        {
                            min_displacement = d;
                            found_place = true;
                        }
                    }
                }
            }

            if let Some(last_node) = segment.nodes.last() {
                let s = self.sync.as_ref().unwrap();
                let ports = last_node
                    .lock().ports().clone();
                for source in ports {
                    let pos = port_position_y_a(s, last_node, &source);
                    let outgoing = source
                        .lock().outgoing_edges().clone();
                    for edge in outgoing {
                        let target = edge.lock().target();
                        let Some(target) = target else {
                            continue;
                        };
                        let other_node =
                            target.lock().node();
                        let Some(other_node) = other_node else {
                            continue;
                        };
                        let d = port_position_y_a(s, &other_node, &target) - pos;
                        if d.abs() < min_displacement.abs()
                            && d.abs()
                                < if d < 0.0 {
                                    min_room_above
                                } else {
                                    min_room_below
                                }
                        {
                            min_displacement = d;
                            found_place = true;
                        }
                    }
                }
            }

            if found_place && min_displacement != 0.0 {
                for node in &segment.nodes {
                    {
                        let mut node_guard = node.lock();
                        node_guard.shape().position().y += min_displacement;
                    }
                }
            }
        }
    }
}

impl Default for LinearSegmentsNodePlacer {
    fn default() -> Self {
        Self::new()
    }
}

impl ILayoutPhase<LayeredPhases, LGraph> for LinearSegmentsNodePlacer {
    fn process(&mut self, graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Linear segments node placement", 1.0);

        let spacings = graph
            .get_property(InternalProperties::SPACINGS)
            .unwrap_or_else(|| panic!("Missing spacings configuration for linear segments"));
        self.spacings = Some(spacings);

        self.sort_linear_segments(graph, monitor);
        // Build arena after sort_linear_segments has assigned sequential node IDs
        self.sync = Some(ArenaSync::from_lgraph(graph));
        self.create_unbalanced_placement(graph);
        self.balance_placement(graph);
        self.post_process(graph);

        self.linear_segments.clear();
        self.spacings = None;
        self.sync = None;
        monitor.done();
    }

    fn get_layout_processor_configuration(
        &self,
        graph: &LGraph,
    ) -> Option<LayoutProcessorConfiguration<LayeredPhases, LGraph>> {
        if graph
            .get_property(InternalProperties::GRAPH_PROPERTIES)
            .is_some_and(|props| props.contains(&GraphProperties::ExternalPorts))
        {
            Some(LayoutProcessorConfiguration::create_from(
                &HIERARCHY_PROCESSING_ADDITIONS,
            ))
        } else {
            None
        }
    }
}

fn region_index(segments: &[LinearSegment], index: usize) -> usize {
    let mut current = index;
    while let Some(next) = segments[current].ref_segment {
        current = next;
    }
    current
}

fn node_id(node: &LNodeRef) -> i32 {
    let mut node_guard = node.lock();
    node_guard.shape().graph_element().id
}

fn set_node_id(node: &LNodeRef, value: i32) {
    {
        let mut node_guard = node.lock();
        node_guard.shape().graph_element().id = value;
    }
}

fn layer_index(node: &LNodeRef) -> usize {
    node.lock().layer()
        .map(|layer| {
            let mut layer_guard = layer.lock();
            layer_guard.graph_element().id as usize
        })
        .unwrap_or(0)
}

fn node_index(node: &LNodeRef) -> usize {
    node.lock().index()
        .unwrap_or(0)
}

fn node_position_y(node: &LNodeRef) -> f64 {
    node.lock().shape().position_ref().y
}

fn node_size_y_a(sync: &ArenaSync, node: &LNodeRef) -> f64 {
    sync.arena().node_size(sync.node_id(node).unwrap()).y
}

fn node_margin_top_a(sync: &ArenaSync, node: &LNodeRef) -> f64 {
    sync.arena().node_margin(sync.node_id(node).unwrap()).top
}

fn node_margin_bottom_a(sync: &ArenaSync, node: &LNodeRef) -> f64 {
    sync.arena().node_margin(sync.node_id(node).unwrap()).bottom
}

fn port_position_y_a(sync: &ArenaSync, node: &LNodeRef, port: &LPortRef) -> f64 {
    let node_pos = node_position_y(node);
    let pid = sync.port_id(port).unwrap();
    let port_pos = sync.arena().port_pos(pid).y;
    let anchor = sync.arena().port_anchor(pid).y;
    node_pos + port_pos + anchor
}
