#![allow(clippy::mutable_key_type)]

use std::collections::{HashMap, HashSet};

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::options::alignment::Alignment;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::IElkProgressMonitor;
use org_eclipse_elk_graph::org::eclipse::elk::graph::properties::Property;

use crate::org::eclipse::elk::alg::layered::graph::{
    index_of_arc, LEdge, LGraph, LGraphUtil, LNodeRef, LPortRef, LayerRef, NodeRefKey, NodeType,
};
use crate::org::eclipse::elk::alg::layered::options::{
    CenterEdgeLabelPlacementStrategy, InternalProperties, LayeredOptions, PortType,
};
use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;
use std::sync::LazyLock;

pub static INCLUDE_LABEL: LazyLock<Property<bool>> =
    LazyLock::new(|| Property::with_default("edgelabelcenterednessanalysis.includelabel", false));

const STRATEGIES: [CenterEdgeLabelPlacementStrategy; 6] = [
    CenterEdgeLabelPlacementStrategy::MedianLayer,
    CenterEdgeLabelPlacementStrategy::TailLayer,
    CenterEdgeLabelPlacementStrategy::HeadLayer,
    CenterEdgeLabelPlacementStrategy::SpaceEfficientLayer,
    CenterEdgeLabelPlacementStrategy::WidestLayer,
    CenterEdgeLabelPlacementStrategy::CenterLayer,
];

pub struct LabelDummySwitcher {
    layer_widths: Vec<f64>,
    layers: Vec<LayerRef>,
    min_space_between_layers: f64,
}

impl Default for LabelDummySwitcher {
    fn default() -> Self {
        Self {
            layer_widths: Vec::new(),
            layers: Vec::new(),
            min_space_between_layers: 0.0,
        }
    }
}

impl ILayoutProcessor<LGraph> for LabelDummySwitcher {
    fn process(&mut self, layered_graph: &mut LGraph, monitor: &mut dyn IElkProgressMonitor) {
        monitor.begin("Label dummy switching", 1.0);
        if ElkTrace::global().label_dummy_switcher {
            eprintln!(
                "label-dummy-switcher: start layers={}",
                layered_graph.layers().len()
            );
        }

        let default_strategy = layered_graph
            .get_property(LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY)
            .unwrap_or(CenterEdgeLabelPlacementStrategy::MedianLayer);
        let direction = layered_graph
            .get_property(LayeredOptions::DIRECTION)
            .unwrap_or(Direction::Undefined);
        self.layers = layered_graph.layers().clone();
        let edge_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_EDGE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0)
            * 2.0;
        let node_node_spacing = layered_graph
            .get_property(LayeredOptions::SPACING_NODE_NODE_BETWEEN_LAYERS)
            .unwrap_or(0.0);
        self.min_space_between_layers = edge_node_spacing.max(node_node_spacing);

        assign_ids_to_layers(layered_graph);
        if ElkTrace::global().label_dummy_switcher {
            eprintln!("label-dummy-switcher: assigned layer ids");
        }

        let mut label_dummy_infos = gather_label_dummy_infos(layered_graph, default_strategy);
        if ElkTrace::global().label_dummy_switcher {
            let total: usize = label_dummy_infos.values().map(|list| list.len()).sum();
            eprintln!(
                "label-dummy-switcher: gathered label dummies total={}",
                total
            );
        }

        self.layer_widths = vec![0.0; self.layers.len()];

        for strategy in STRATEGIES {
            if strategy.uses_label_size_information() {
                if let Some(infos) = label_dummy_infos.get(&strategy) {
                    if !infos.is_empty() {
                        if ElkTrace::global().label_dummy_switcher {
                            eprintln!(
                                "label-dummy-switcher: calculating layer widths for {:?}",
                                strategy
                            );
                        }
                        self.calculate_layer_widths(direction);
                        break;
                    }
                }
            }
        }

        for strategy in STRATEGIES {
            if !strategy.uses_label_size_information() {
                if let Some(infos) = label_dummy_infos.get_mut(&strategy) {
                    self.process_strategy(infos);
                }
            }
        }

        for strategy in STRATEGIES {
            if strategy.uses_label_size_information() {
                if let Some(infos) = label_dummy_infos.get_mut(&strategy) {
                    self.process_strategy(infos);
                }
            }
        }

        self.layer_widths.clear();
        self.layers.clear();
        self.min_space_between_layers = 0.0;
        monitor.done();
    }
}

fn assign_ids_to_layers(layered_graph: &LGraph) {
    for (index, layer) in layered_graph.layers().iter().enumerate() {
        if let Some(mut layer_guard) = layer.lock_ok() {
            layer_guard.graph_element().id = index as i32;
        }
    }
}

fn gather_label_dummy_infos(
    layered_graph: &LGraph,
    default_strategy: CenterEdgeLabelPlacementStrategy,
) -> HashMap<CenterEdgeLabelPlacementStrategy, Vec<LabelDummyInfo>> {
    let mut infos: HashMap<CenterEdgeLabelPlacementStrategy, Vec<LabelDummyInfo>> = HashMap::new();

    for strategy in STRATEGIES {
        infos.insert(strategy, Vec::new());
    }

    for layer in layered_graph.layers() {
        let nodes = layer
            .lock_ok()
            .map(|layer_guard| layer_guard.nodes().clone())
            .unwrap_or_default();
        for node in nodes {
            let is_label = node
                .lock_ok()
                .map(|node_guard| node_guard.node_type() == NodeType::Label)
                .unwrap_or(false);
            if !is_label {
                continue;
            }

            let dummy_info = LabelDummyInfo::new(node, default_strategy);
            if let Some(list) = infos.get_mut(&dummy_info.placement_strategy) {
                list.push(dummy_info);
            }
        }
    }

    infos
}

impl LabelDummySwitcher {
    fn calculate_layer_widths(&mut self, direction: Direction) {
        for layer in &self.layers {
            let (layer_id, node_count) = layer
                .lock_ok()
                .map(|mut layer_guard| (layer_guard.graph_element().id, layer_guard.nodes().len()))
                .unwrap_or((0, 0));
            if ElkTrace::global().label_dummy_switcher {
                eprintln!(
                    "label-dummy-switcher: layer id={} nodes={}",
                    layer_id, node_count
                );
            }
            if layer_id >= 0 {
                if ElkTrace::global().label_dummy_switcher {
                    eprintln!(
                        "label-dummy-switcher: layer id={} width calc start",
                        layer_id
                    );
                }
                let width = LGraphUtil::find_max_non_dummy_node_width(layer, direction, false);
                if ElkTrace::global().label_dummy_switcher {
                    eprintln!(
                        "label-dummy-switcher: layer id={} width calc done width={}",
                        layer_id, width
                    );
                }
                if let Some(entry) = self.layer_widths.get_mut(layer_id as usize) {
                    *entry = width;
                }
            }
        }
    }

    fn process_strategy(&mut self, label_dummy_infos: &mut [LabelDummyInfo]) {
        if label_dummy_infos.is_empty() {
            return;
        }
        if ElkTrace::global().label_dummy_switcher {
            eprintln!(
                "label-dummy-switcher: strategy {:?} count={}",
                label_dummy_infos[0].placement_strategy,
                label_dummy_infos.len()
            );
        }

        if label_dummy_infos[0].placement_strategy
            == CenterEdgeLabelPlacementStrategy::SpaceEfficientLayer
        {
            self.compute_space_efficient_assignment(label_dummy_infos);
            return;
        }

        for label_dummy_info in label_dummy_infos.iter() {
            if ElkTrace::global().label_dummy_switcher {
                eprintln!(
                    "label-dummy-switcher: label left={} right={} left_dummies={} right_dummies={}",
                    label_dummy_info.leftmost_layer_id,
                    label_dummy_info.rightmost_layer_id,
                    label_dummy_info.left_long_edge_dummies.len(),
                    label_dummy_info.right_long_edge_dummies.len()
                );
            }
            match label_dummy_info.placement_strategy {
                CenterEdgeLabelPlacementStrategy::CenterLayer => {
                    let target = self.find_center_layer_target_id(label_dummy_info);
                    self.assign_layer(label_dummy_info, target);
                }
                CenterEdgeLabelPlacementStrategy::MedianLayer => {
                    let target = self.find_median_layer_target_id(label_dummy_info);
                    self.assign_layer(label_dummy_info, target);
                }
                CenterEdgeLabelPlacementStrategy::WidestLayer => {
                    let target = self.find_widest_layer_target_id(label_dummy_info);
                    self.assign_layer(label_dummy_info, target);
                }
                CenterEdgeLabelPlacementStrategy::HeadLayer => {
                    self.set_end_layer_node_alignment(label_dummy_info);
                    let target = self.find_end_layer_target_id(label_dummy_info, true);
                    self.assign_layer(label_dummy_info, target);
                }
                CenterEdgeLabelPlacementStrategy::TailLayer => {
                    self.set_end_layer_node_alignment(label_dummy_info);
                    let target = self.find_end_layer_target_id(label_dummy_info, false);
                    self.assign_layer(label_dummy_info, target);
                }
                CenterEdgeLabelPlacementStrategy::SpaceEfficientLayer => {}
            }

            self.update_long_edge_source_label_dummy_info(label_dummy_info);
        }
    }

    fn find_widest_layer_target_id(&self, label_dummy_info: &LabelDummyInfo) -> usize {
        let mut widest_layer = label_dummy_info.leftmost_layer_id;
        for index in (widest_layer + 1)..=label_dummy_info.rightmost_layer_id {
            let current = self.layer_widths.get(index).copied().unwrap_or(0.0);
            let widest = self.layer_widths.get(widest_layer).copied().unwrap_or(0.0);
            if current > widest {
                widest_layer = index;
            }
        }
        widest_layer
    }

    fn find_center_layer_target_id(&self, label_dummy_info: &LabelDummyInfo) -> usize {
        let layer_width_sums = self.compute_layer_width_sums(label_dummy_info);
        let threshold = layer_width_sums.last().copied().unwrap_or(0.0) / 2.0;

        for (index, value) in layer_width_sums.iter().enumerate() {
            if *value >= threshold {
                return label_dummy_info.leftmost_layer_id + index;
            }
        }

        label_dummy_info.leftmost_layer_id + label_dummy_info.left_long_edge_dummies.len()
    }

    fn compute_layer_width_sums(&self, label_dummy_info: &LabelDummyInfo) -> Vec<f64> {
        let min_space_between_layers = self.min_space_between_layers;

        let mut sums = vec![0.0; label_dummy_info.total_dummy_count()];
        let mut current_sum = -min_space_between_layers;
        let mut index = 0usize;

        for left_dummy in &label_dummy_info.left_long_edge_dummies {
            let layer_id = node_layer_id(left_dummy).unwrap_or(0);
            current_sum +=
                self.layer_widths.get(layer_id).copied().unwrap_or(0.0) + min_space_between_layers;
            sums[index] = current_sum;
            index += 1;
        }

        let label_layer_id = node_layer_id(&label_dummy_info.label_dummy).unwrap_or(0);
        current_sum += self
            .layer_widths
            .get(label_layer_id)
            .copied()
            .unwrap_or(0.0)
            + min_space_between_layers;
        if index < sums.len() {
            sums[index] = current_sum;
            index += 1;
        }

        for right_dummy in &label_dummy_info.right_long_edge_dummies {
            let layer_id = node_layer_id(right_dummy).unwrap_or(0);
            current_sum +=
                self.layer_widths.get(layer_id).copied().unwrap_or(0.0) + min_space_between_layers;
            if index < sums.len() {
                sums[index] = current_sum;
                index += 1;
            }
        }

        sums
    }

    fn find_median_layer_target_id(&self, label_dummy_info: &LabelDummyInfo) -> usize {
        let layers = label_dummy_info.total_dummy_count();
        let lower_median = (layers - 1) / 2;
        label_dummy_info.leftmost_layer_id + lower_median
    }

    fn find_end_layer_target_id(
        &self,
        label_dummy_info: &LabelDummyInfo,
        head_layer: bool,
    ) -> usize {
        let reversed = self.is_part_of_reversed_edge(label_dummy_info);
        if (head_layer && !reversed) || (!head_layer && reversed) {
            label_dummy_info.rightmost_layer_id
        } else {
            label_dummy_info.leftmost_layer_id
        }
    }

    fn set_end_layer_node_alignment(&self, label_dummy_info: &LabelDummyInfo) {
        let is_head_label =
            label_dummy_info.placement_strategy == CenterEdgeLabelPlacementStrategy::HeadLayer;
        let reversed = self.is_part_of_reversed_edge(label_dummy_info);
        let alignment = if (is_head_label && !reversed) || (!is_head_label && reversed) {
            Alignment::Right
        } else {
            Alignment::Left
        };

        if let Some(mut node_guard) = label_dummy_info.label_dummy.lock_ok() {
            node_guard.set_property(LayeredOptions::ALIGNMENT, Some(alignment));
        }
    }

    fn is_part_of_reversed_edge(&self, label_dummy_info: &LabelDummyInfo) -> bool {
        let incoming = label_dummy_info
            .label_dummy
            .lock_ok()
            .and_then(|node_guard| node_guard.incoming_edges().first().cloned());
        let outgoing = label_dummy_info
            .label_dummy
            .lock_ok()
            .and_then(|node_guard| node_guard.outgoing_edges().first().cloned());

        let incoming_reversed = incoming
            .as_ref()
            .and_then(|edge| {
                edge.lock_ok().and_then(|mut edge_guard| {
                    edge_guard.get_property(InternalProperties::REVERSED)
                })
            })
            .unwrap_or(false);
        let outgoing_reversed = outgoing
            .as_ref()
            .and_then(|edge| {
                edge.lock_ok().and_then(|mut edge_guard| {
                    edge_guard.get_property(InternalProperties::REVERSED)
                })
            })
            .unwrap_or(false);

        incoming_reversed || outgoing_reversed
    }

    fn compute_space_efficient_assignment(&mut self, label_dummy_infos: &mut [LabelDummyInfo]) {
        let mut non_trivial_labels = self.perform_trivial_assignments(label_dummy_infos);
        if non_trivial_labels.is_empty() {
            return;
        }

        non_trivial_labels.sort_by(|info1, info2| {
            let width1 = info1.label_dummy_width();
            let width2 = info2.label_dummy_width();
            width2.total_cmp(&width1)
        });

        for label_index in 0..non_trivial_labels.len() {
            let target = self.find_potentially_widest_layer(&non_trivial_labels, label_index);
            self.assign_layer(&non_trivial_labels[label_index], target);
        }
    }

    fn perform_trivial_assignments(
        &mut self,
        label_dummy_infos: &[LabelDummyInfo],
    ) -> Vec<LabelDummyInfo> {
        let mut remaining = Vec::new();
        for info in label_dummy_infos {
            if info.leftmost_layer_id == info.rightmost_layer_id {
                self.assign_layer(info, info.leftmost_layer_id);
            } else if !self.assign_to_wider_layer(info) {
                remaining.push(info.clone());
            }
        }
        remaining
    }

    fn assign_to_wider_layer(&mut self, label_dummy_info: &LabelDummyInfo) -> bool {
        let dummy_width = label_dummy_info.label_dummy_width();

        for layer_index in label_dummy_info.leftmost_layer_id..=label_dummy_info.rightmost_layer_id
        {
            if let Some(layer) = self.layers.get(layer_index) {
                let layer_width = layer
                    .lock_ok()
                    .map(|layer_guard| layer_guard.size_ref().x)
                    .unwrap_or(0.0);
                if layer_width >= dummy_width {
                    self.assign_layer(label_dummy_info, layer_index);
                    return true;
                }
            }
        }

        false
    }

    fn find_potentially_widest_layer(
        &self,
        label_dummy_infos: &[LabelDummyInfo],
        label_index: usize,
    ) -> usize {
        let label_dummy_info = &label_dummy_infos[label_index];
        let label_dummy_width = label_dummy_info.label_dummy_width();

        let mut widest_layer_index = label_dummy_info.leftmost_layer_id;
        let mut widest_layer_width = 0.0;

        for layer in label_dummy_info.leftmost_layer_id..=label_dummy_info.rightmost_layer_id {
            if label_dummy_width <= self.layer_widths.get(layer).copied().unwrap_or(0.0) {
                return layer;
            }

            let mut potential_width = self.layer_widths.get(layer).copied().unwrap_or(0.0);
            let mut largest_unassigned = None;

            for candidate in label_dummy_infos.iter().skip(label_index + 1) {
                if candidate.leftmost_layer_id <= layer && candidate.rightmost_layer_id >= layer {
                    largest_unassigned = Some(candidate);
                }
            }

            if let Some(candidate) = largest_unassigned {
                potential_width = potential_width.max(candidate.label_dummy_width());
            }

            if potential_width > widest_layer_width {
                widest_layer_index = layer;
                widest_layer_width = potential_width;
            }
        }

        widest_layer_index
    }

    fn assign_layer(&mut self, label_dummy_info: &LabelDummyInfo, target_layer_index: usize) {
        let current_layer_index =
            label_dummy_info.leftmost_layer_id + label_dummy_info.left_long_edge_dummies.len();

        if ElkTrace::global().label_dummy_switcher {
            eprintln!(
                "label-dummy-switcher: assign target={} current={} left={} right={}",
                target_layer_index,
                current_layer_index,
                label_dummy_info.leftmost_layer_id,
                label_dummy_info.rightmost_layer_id
            );
        }
        if target_layer_index != current_layer_index {
            let swap_index = target_layer_index - label_dummy_info.leftmost_layer_id;
            let swap_node = label_dummy_info.ith_dummy_node(swap_index);
            self.swap_nodes(&label_dummy_info.label_dummy, &swap_node);
        }

        let new_layer_id =
            node_layer_id(&label_dummy_info.label_dummy).unwrap_or(target_layer_index);
        let dummy_width = label_dummy_info.label_dummy_width();
        if let Some(width) = self.layer_widths.get_mut(new_layer_id) {
            if dummy_width > *width {
                *width = dummy_width;
            }
        }

        let labels = label_dummy_info
            .label_dummy
            .lock_ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::REPRESENTED_LABELS)
            })
            .unwrap_or_default();
        for label in labels {
            if let Some(mut label_guard) = label.lock_ok() {
                label_guard.set_property(&INCLUDE_LABEL, Some(true));
            }
        }
        if ElkTrace::global().label_dummy_switcher {
            eprintln!(
                "label-dummy-switcher: assign done target={}",
                target_layer_index
            );
        }
    }

    fn swap_nodes(&self, label_dummy: &LNodeRef, long_edge_dummy: &LNodeRef) {
        if ElkTrace::global().label_dummy_switcher {
            eprintln!("label-dummy-switcher: swap start");
        }
        let (layer1, layer2) = match (
            label_dummy
                .lock_ok()
                .and_then(|node_guard| node_guard.layer()),
            long_edge_dummy
                .lock_ok()
                .and_then(|node_guard| node_guard.layer()),
        ) {
            (Some(layer1), Some(layer2)) => (layer1, layer2),
            _ => return,
        };

        let dummy1_pos = layer1
            .lock_ok()
            .and_then(|layer_guard| index_of_arc(layer_guard.nodes(), label_dummy));
        let dummy2_pos = layer2
            .lock_ok()
            .and_then(|layer_guard| index_of_arc(layer_guard.nodes(), long_edge_dummy));
        let (dummy1_pos, dummy2_pos) = match (dummy1_pos, dummy2_pos) {
            (Some(pos1), Some(pos2)) => (pos1, pos2),
            _ => return,
        };

        let (input1, output1) = match label_dummy.lock_ok() {
            Some(node_guard) => {
                let input = node_guard.ports_by_type(PortType::Input).first().cloned();
                let output = node_guard.ports_by_type(PortType::Output).first().cloned();
                (input, output)
            }
            None => (None, None),
        };
        let (input2, output2) = match long_edge_dummy.lock_ok() {
            Some(node_guard) => {
                let input = node_guard.ports_by_type(PortType::Input).first().cloned();
                let output = node_guard.ports_by_type(PortType::Output).first().cloned();
                (input, output)
            }
            None => (None, None),
        };

        let (input1, output1, input2, output2) = match (input1, output1, input2, output2) {
            (Some(input1), Some(output1), Some(input2), Some(output2)) => {
                (input1, output1, input2, output2)
            }
            _ => return,
        };

        let incoming_edges1 = port_incoming_edges(&input1);
        let outgoing_edges1 = port_outgoing_edges(&output1);
        let incoming_edges2 = port_incoming_edges(&input2);
        let outgoing_edges2 = port_outgoing_edges(&output2);

        crate::org::eclipse::elk::alg::layered::graph::LNode::set_layer_at_index(
            label_dummy,
            dummy2_pos,
            Some(layer2.clone()),
        );
        for edge in incoming_edges2 {
            LEdge::set_target(&edge, Some(input1.clone()));
        }
        for edge in outgoing_edges2 {
            LEdge::set_source(&edge, Some(output1.clone()));
        }

        crate::org::eclipse::elk::alg::layered::graph::LNode::set_layer_at_index(
            long_edge_dummy,
            dummy1_pos,
            Some(layer1.clone()),
        );
        for edge in incoming_edges1 {
            LEdge::set_target(&edge, Some(input2.clone()));
        }
        for edge in outgoing_edges1 {
            LEdge::set_source(&edge, Some(output2.clone()));
        }
        if ElkTrace::global().label_dummy_switcher {
            eprintln!("label-dummy-switcher: swap done");
        }
    }

    fn update_long_edge_source_label_dummy_info(&self, label_dummy_info: &LabelDummyInfo) {
        self.do_update_long_edge_label_dummy_info(
            &label_dummy_info.label_dummy,
            previous_long_edge_node,
            true,
        );
    }

    fn do_update_long_edge_label_dummy_info<F>(
        &self,
        label_dummy: &LNodeRef,
        next_element: F,
        value: bool,
    ) where
        F: Fn(&LNodeRef) -> Option<LNodeRef>,
    {
        let mut long_edge_dummy = match next_element(label_dummy) {
            Some(node) => node,
            None => return,
        };
        let mut visited: HashSet<NodeRefKey> = HashSet::new();

        while node_type(&long_edge_dummy) == NodeType::LongEdge {
            if !visited.insert(NodeRefKey(long_edge_dummy.clone())) {
                break;
            }
            if let Some(mut node_guard) = long_edge_dummy.lock_ok() {
                node_guard.set_property(
                    InternalProperties::LONG_EDGE_BEFORE_LABEL_DUMMY,
                    Some(value),
                );
            }

            long_edge_dummy = match next_element(&long_edge_dummy) {
                Some(node) => node,
                None => break,
            };
        }
    }
}

fn port_incoming_edges(
    port: &LPortRef,
) -> Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef> {
    port.lock_ok()
        .map(|port_guard| LGraphUtil::to_edge_array(port_guard.incoming_edges()))
        .unwrap_or_default()
}

fn port_outgoing_edges(
    port: &LPortRef,
) -> Vec<crate::org::eclipse::elk::alg::layered::graph::LEdgeRef> {
    port.lock_ok()
        .map(|port_guard| LGraphUtil::to_edge_array(port_guard.outgoing_edges()))
        .unwrap_or_default()
}

fn node_layer_id(node: &LNodeRef) -> Option<usize> {
    node.lock_ok()
        .and_then(|node_guard| node_guard.layer())
        .and_then(|layer| {
            layer
                .lock_ok()
                .map(|mut layer_guard| layer_guard.graph_element().id as usize)
        })
}

fn node_type(node: &LNodeRef) -> NodeType {
    node.lock_ok()
        .map(|node_guard| node_guard.node_type())
        .unwrap_or(NodeType::Normal)
}

fn previous_long_edge_node(node: &LNodeRef) -> Option<LNodeRef> {
    node.lock_ok()
        .and_then(|node_guard| node_guard.incoming_edges().first().cloned())
        .and_then(|edge| edge.lock_ok().and_then(|edge_guard| edge_guard.source()))
        .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()))
}

#[derive(Clone)]
struct LabelDummyInfo {
    label_dummy: LNodeRef,
    placement_strategy: CenterEdgeLabelPlacementStrategy,
    left_long_edge_dummies: Vec<LNodeRef>,
    right_long_edge_dummies: Vec<LNodeRef>,
    leftmost_layer_id: usize,
    rightmost_layer_id: usize,
}

impl LabelDummyInfo {
    fn new(label_dummy: LNodeRef, default_strategy: CenterEdgeLabelPlacementStrategy) -> Self {
        let mut info = Self {
            label_dummy,
            placement_strategy: default_strategy,
            left_long_edge_dummies: Vec::new(),
            right_long_edge_dummies: Vec::new(),
            leftmost_layer_id: 0,
            rightmost_layer_id: 0,
        };

        info.gather_left_long_edge_dummies();
        info.gather_right_long_edge_dummies();

        info.leftmost_layer_id = info
            .left_long_edge_dummies
            .first()
            .and_then(node_layer_id)
            .or_else(|| node_layer_id(&info.label_dummy))
            .unwrap_or(0);
        info.rightmost_layer_id = info
            .right_long_edge_dummies
            .last()
            .and_then(node_layer_id)
            .or_else(|| node_layer_id(&info.label_dummy))
            .unwrap_or(info.leftmost_layer_id);

        let represented_labels = info
            .label_dummy
            .lock_ok()
            .and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::REPRESENTED_LABELS)
            })
            .unwrap_or_default();
        for label in represented_labels {
            let override_strategy = label.lock_ok().and_then(|mut label_guard| {
                if label_guard
                    .shape()
                    .graph_element()
                    .properties()
                    .has_property(LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY)
                {
                    label_guard
                        .get_property(LayeredOptions::EDGE_LABELS_CENTER_LABEL_PLACEMENT_STRATEGY)
                } else {
                    None
                }
            });
            if let Some(strategy) = override_strategy {
                info.placement_strategy = strategy;
                break;
            }
        }

        info
    }

    fn gather_left_long_edge_dummies(&mut self) {
        let mut source = self.label_dummy.clone();
        let mut visited: HashSet<NodeRefKey> = HashSet::new();
        while let Some(next) = previous_long_edge_node(&source) {
            if !visited.insert(NodeRefKey(next.clone())) {
                break;
            }
            if node_type(&next) == NodeType::LongEdge {
                self.left_long_edge_dummies.push(next.clone());
                source = next;
            } else {
                break;
            }
        }
        self.left_long_edge_dummies.reverse();
    }

    fn gather_right_long_edge_dummies(&mut self) {
        let mut target = self.label_dummy.clone();
        let mut visited: HashSet<NodeRefKey> = HashSet::new();
        loop {
            let next = target
                .lock_ok()
                .and_then(|node_guard| node_guard.outgoing_edges().first().cloned())
                .and_then(|edge| edge.lock_ok().and_then(|edge_guard| edge_guard.target()))
                .and_then(|port| port.lock_ok().and_then(|port_guard| port_guard.node()));
            let Some(next) = next else {
                break;
            };
            if !visited.insert(NodeRefKey(next.clone())) {
                break;
            }
            if node_type(&next) == NodeType::LongEdge {
                self.right_long_edge_dummies.push(next.clone());
                target = next;
            } else {
                break;
            }
        }
    }

    fn total_dummy_count(&self) -> usize {
        self.rightmost_layer_id
            .saturating_sub(self.leftmost_layer_id)
            + 1
    }

    fn ith_dummy_node(&self, index: usize) -> LNodeRef {
        if index < self.left_long_edge_dummies.len() {
            self.left_long_edge_dummies[index].clone()
        } else if index == self.left_long_edge_dummies.len() {
            self.label_dummy.clone()
        } else {
            let right_index = index - self.left_long_edge_dummies.len() - 1;
            self.right_long_edge_dummies[right_index].clone()
        }
    }

    fn label_dummy_width(&self) -> f64 {
        self.label_dummy
            .lock_ok()
            .map(|mut node_guard| node_guard.shape().size_ref().x)
            .unwrap_or(0.0)
    }
}
