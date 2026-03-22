use std::collections::VecDeque;

use rustc_hash::FxHashMap;
use std::sync::Arc;

use org_eclipse_elk_core::org::eclipse::elk::core::util::elk_trace::ElkTrace;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_heuristic::BarycenterState;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;
use crate::org::eclipse::elk::alg::layered::p3order::cross_min_snapshot::CrossMinSnapshot;

pub struct ForsterConstraintResolver {
    constraints_between_non_dummies: bool,
    layout_units: FxHashMap<usize, Vec<LNodeRef>>,
    pub barycenter_states: Vec<Vec<Option<BarycenterState>>>,
    constraint_groups: Vec<Vec<Option<ConstraintGroupId>>>,
    constraint_group_arena: Vec<ConstraintGroup>,
    snapshot: Option<Arc<CrossMinSnapshot>>,
}

type ConstraintGroupId = usize;

impl ForsterConstraintResolver {
    pub fn new(
        _current_node_order: &[Vec<LNodeRef>],
        constraints_between_non_dummies: bool,
    ) -> Self {
        ForsterConstraintResolver {
            constraints_between_non_dummies,
            layout_units: FxHashMap::default(),
            barycenter_states: Vec::new(),
            constraint_groups: Vec::new(),
            constraint_group_arena: Vec::new(),
            snapshot: None,
        }
    }

    pub fn set_snapshot(&mut self, snapshot: Arc<CrossMinSnapshot>) {
        self.snapshot = Some(snapshot);
    }

    #[inline]
    fn snap_node_id(&self, node: &LNodeRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.node_id(node) as usize
        } else {
            node_id(node)
        }
    }

    #[inline]
    fn snap_layer_index(&self, node: &LNodeRef) -> usize {
        if let Some(ref snap) = self.snapshot {
            snap.node_layer_index(node) as usize
        } else {
            layer_index(node)
        }
    }

    #[inline]
    fn snap_node_type(&self, node: &LNodeRef) -> NodeType {
        if let Some(ref snap) = self.snapshot {
            snap.node_type_of(snap.node_flat_index(node))
        } else {
            node.lock().node_type()
        }
    }

    fn add_group(&mut self, group: ConstraintGroup) -> ConstraintGroupId {
        let group_id = self.constraint_group_arena.len();
        self.constraint_group_arena.push(group);
        group_id
    }

    fn group(&self, group_id: ConstraintGroupId) -> Option<&ConstraintGroup> {
        self.constraint_group_arena.get(group_id)
    }

    fn group_mut(&mut self, group_id: ConstraintGroupId) -> Option<&mut ConstraintGroup> {
        self.constraint_group_arena.get_mut(group_id)
    }

    pub fn process_constraints(&mut self, nodes: &mut Vec<LNodeRef>) {
        if self.constraints_between_non_dummies {
            self.process_constraints_internal(nodes, true);
            for node in nodes.iter() {
                self.init_node_level(node, false);
            }
        }
        self.process_constraints_internal(nodes, false);
    }

    fn process_constraints_internal(
        &mut self,
        nodes: &mut Vec<LNodeRef>,
        only_between_normal_nodes: bool,
    ) {
        let mut groups: Vec<ConstraintGroupId> = Vec::with_capacity(nodes.len());
        for node in nodes.iter() {
            let group = self.group_of(node);
            groups.push(group);
        }

        let trace = ElkTrace::global().forster_groups
            && groups
                .iter()
                .copied()
                .any(|group_id| group_contains_pump(self, group_id));
        if trace {
            eprintln!(
                "crossmin: forster start only_between_normal_nodes={} groups=[{}]",
                only_between_normal_nodes,
                format_group_list(self, &groups)
            );
        }

        self.build_constraints_graph(&groups, only_between_normal_nodes);

        while let Some((first, second)) = self.find_violated_constraint(&groups) {
            if trace {
                eprintln!(
                    "crossmin: forster violated first={} second={}",
                    format_group(self, first),
                    format_group(self, second)
                );
            }
            self.handle_violated_constraint(first, second, &mut groups);
            if trace {
                eprintln!(
                    "crossmin: forster after_merge groups=[{}]",
                    format_group_list(self, &groups)
                );
            }
        }

        nodes.clear();
        for group_id in groups.iter().copied() {
            let group_barycenter = self.group_barycenter(group_id);
            let group_nodes = self
                .group(group_id)
                .map(|group| group.nodes.clone())
                .unwrap_or_default();
            for node in group_nodes {
                nodes.push(node.clone());
                let (li, ni) = (self.snap_layer_index(&node), self.snap_node_id(&node));
                if let Some(state) = self.barycenter_states
                    .get_mut(li).and_then(|l| l.get_mut(ni)).and_then(|o| o.as_mut())
                {
                    state.barycenter = group_barycenter;
                }
            }
        }
    }

    fn build_constraints_graph(
        &mut self,
        groups: &[ConstraintGroupId],
        only_between_normal_nodes: bool,
    ) {
        let trace = ElkTrace::global().forster_groups
            && groups
                .iter()
                .copied()
                .any(|group_id| group_contains_pump(self, group_id));

        for group_id in groups.iter().copied() {
            if let Some(group) = self.group_mut(group_id) {
                group.reset_outgoing_constraints();
                group.incoming_constraints_count = 0;
            }
        }

        let mut last_non_dummy_node: Option<LNodeRef> = None;
        for group_id in groups.iter().copied() {
            let node = match self.group(group_id).map(|group| group.single_node().clone()) {
                Some(node) => node,
                None => continue,
            };
            if only_between_normal_nodes {
                let node_type = self.snap_node_type(&node);
                if node_type != NodeType::Normal {
                    continue;
                }
            }

            let successors = node
                .lock()
                .get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                .unwrap_or_default();
            for successor in successors {
                if only_between_normal_nodes {
                    let successor_type = self.snap_node_type(&successor);
                    if successor_type != NodeType::Normal {
                        continue;
                    }
                }
                let successor_group = self.group_of(&successor);
                if let Some(group) = self.group_mut(group_id) {
                    group.outgoing_constraints_mut().push(successor_group);
                }
                if let Some(successor_group_data) = self.group_mut(successor_group) {
                    successor_group_data.incoming_constraints_count += 1;
                }
            }

            if !only_between_normal_nodes {
                let node_type = self.snap_node_type(&node);
                if node_type == NodeType::Normal {
                    if let Some(last_node) = last_non_dummy_node.clone() {
                        let last_unit_key = node_ptr_id(&last_node);
                        let current_unit_key = node_ptr_id(&node);
                        let last_unit_nodes = self
                            .layout_units
                            .get(&last_unit_key)
                            .cloned()
                            .unwrap_or_default();
                        let current_unit_nodes = self
                            .layout_units
                            .get(&current_unit_key)
                            .cloned()
                            .unwrap_or_default();
                        for last_unit_node in last_unit_nodes {
                            for current_unit_node in current_unit_nodes.iter() {
                                let last_group = self.group_of(&last_unit_node);
                                let current_group = self.group_of(current_unit_node);
                                if let Some(last_group_data) = self.group_mut(last_group) {
                                    last_group_data.outgoing_constraints_mut().push(current_group);
                                }
                                if let Some(current_group_data) = self.group_mut(current_group) {
                                    current_group_data.incoming_constraints_count += 1;
                                }
                            }
                        }
                    }
                    last_non_dummy_node = Some(node.clone());
                }
            }
        }

        if trace {
            for group_id in groups.iter().copied() {
                let (incoming_count, outgoing) = self
                    .group(group_id)
                    .map(|group| {
                        let outgoing = group
                            .outgoing_constraints
                            .iter()
                            .map(|target| format_group(self, *target))
                            .collect::<Vec<_>>()
                            .join(", ");
                        (group.incoming_constraints_count, outgoing)
                    })
                    .unwrap_or((0, String::new()));
                eprintln!(
                    "crossmin: forster graph group={} incoming_count={} outgoing=[{}]",
                    format_group(self, group_id),
                    incoming_count,
                    outgoing
                );
            }
        }
    }

    fn find_violated_constraint(
        &mut self,
        groups: &[ConstraintGroupId],
    ) -> Option<(ConstraintGroupId, ConstraintGroupId)> {
        let mut active_groups: VecDeque<ConstraintGroupId> = VecDeque::new();
        let mut index_map: FxHashMap<ConstraintGroupId, usize> = FxHashMap::default();
        for (index, group_id) in groups.iter().copied().enumerate() {
            index_map.insert(group_id, index);
            if let Some(group) = self.group_mut(group_id) {
                group.reset_incoming_constraints();
                if group.has_outgoing_constraints() && group.incoming_constraints_count == 0 {
                    active_groups.push_back(group_id);
                }
            }
        }

        while let Some(group_id) = active_groups.pop_front() {
            let incoming = self
                .group(group_id)
                .map(|group| group.incoming_constraints.clone())
                .unwrap_or_default();
            if !incoming.is_empty() {
                for predecessor in incoming {
                    let pred_bary = self.group_barycenter(predecessor).unwrap_or(0.0);
                    let group_bary = self.group_barycenter(group_id).unwrap_or(0.0);
                    // Java compares via .floatValue() (f32 truncation)
                    if (pred_bary as f32) == (group_bary as f32) {
                        let pred_index = index_map
                            .get(&predecessor)
                            .copied()
                            .unwrap_or(0);
                        let group_index = index_map.get(&group_id).copied().unwrap_or(0);
                        if pred_index > group_index {
                            return Some((predecessor, group_id));
                        }
                    } else if pred_bary > group_bary {
                        return Some((predecessor, group_id));
                    }
                }
            }

            let outgoing = self
                .group(group_id)
                .map(|group| group.outgoing_constraints.clone())
                .unwrap_or_default();
            for successor_id in outgoing {
                let incoming_count = self
                    .group(successor_id)
                    .map(|successor| successor.incoming_constraints_count)
                    .unwrap_or(0);
                let list_len = if let Some(successor) = self.group_mut(successor_id) {
                    successor.incoming_constraints.insert(0, group_id);
                    successor.incoming_constraints.len()
                } else {
                    0
                };
                if incoming_count == list_len {
                    active_groups.push_back(successor_id);
                }
            }
        }

        None
    }

    fn handle_violated_constraint(
        &mut self,
        first: ConstraintGroupId,
        second: ConstraintGroupId,
        groups: &mut Vec<ConstraintGroupId>,
    ) {
        let new_group = self.merge_groups(first, second);
        let new_barycenter = self.group_barycenter(new_group).unwrap_or(0.0);

        let mut already_inserted = false;
        let mut index = 0usize;
        while index < groups.len() {
            let group = groups[index];
            if group == first || group == second {
                groups.remove(index);
                continue;
            }

            let group_barycenter = self.group_barycenter(group).unwrap_or(0.0);
            if !already_inserted && group_barycenter > new_barycenter {
                groups.insert(index, new_group);
                already_inserted = true;
                continue;
            }

            if let Some(group_data) = self.group_mut(group) {
                if group_data.has_outgoing_constraints() {
                    let outgoing = group_data.outgoing_constraints_mut();
                    let first_removed = remove_group(outgoing, first);
                    let second_removed = remove_group(outgoing, second);
                    if first_removed || second_removed {
                        outgoing.push(new_group);
                        if let Some(new_group_data) = self.group_mut(new_group) {
                            new_group_data.incoming_constraints_count += 1;
                        }
                    }
                }
            }
            index += 1;
        }

        if !already_inserted {
            groups.push(new_group);
        }
    }

    fn merge_groups(
        &mut self,
        group1: ConstraintGroupId,
        group2: ConstraintGroupId,
    ) -> ConstraintGroupId {
        let (nodes1, sum1, deg1, outgoing1) = self
            .group(group1)
            .map(|group| {
                (
                    group.nodes.clone(),
                    group.summed_weight,
                    group.degree,
                    group.outgoing_constraints.clone(),
                )
            })
            .unwrap_or((Vec::new(), 0.0, 0, Vec::new()));
        let (nodes2, sum2, deg2, outgoing2) = self
            .group(group2)
            .map(|group| {
                (
                    group.nodes.clone(),
                    group.summed_weight,
                    group.degree,
                    group.outgoing_constraints.clone(),
                )
            })
            .unwrap_or((Vec::new(), 0.0, 0, Vec::new()));
        let mut nodes = nodes1;
        nodes.extend(nodes2);

        let mut new_group = ConstraintGroup::new_with_nodes(nodes);
        new_group.summed_weight = sum1 + sum2;
        new_group.degree = deg1 + deg2;

        if !outgoing1.is_empty() {
            let mut outgoing = outgoing1.clone();
            remove_group(&mut outgoing, group2);
            for candidate in outgoing2 {
                if candidate == group1 {
                    continue;
                }
                if contains_group(&outgoing, candidate) {
                    if let Some(candidate_group) = self.group_mut(candidate) {
                        candidate_group.incoming_constraints_count =
                            candidate_group.incoming_constraints_count.saturating_sub(1);
                    }
                } else {
                    outgoing.push(candidate);
                }
            }
            new_group.outgoing_constraints = outgoing;
        } else if !outgoing2.is_empty() {
            let mut outgoing = outgoing2.clone();
            remove_group(&mut outgoing, group1);
            new_group.outgoing_constraints = outgoing;
        }

        let barycenter = if new_group.degree > 0 {
            Some(new_group.summed_weight / new_group.degree as f64)
        } else {
            let b1 = self.group_barycenter(group1);
            let b2 = self.group_barycenter(group2);
            match (b1, b2) {
                (Some(b1), Some(b2)) => Some((b1 + b2) / 2.0),
                (Some(b1), None) => Some(b1),
                (None, Some(b2)) => Some(b2),
                _ => None,
            }
        };

        let new_group_id = self.add_group(new_group);
        self.set_group_barycenter(new_group_id, barycenter);
        new_group_id
    }

    fn group_of(&self, node: &LNodeRef) -> ConstraintGroupId {
        let layer_index = self.snap_layer_index(node);
        let node_index = self.snap_node_id(node);
        self.constraint_groups
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
            .and_then(|group| *group)
            .expect("constraint group missing")
    }

    fn group_barycenter(&self, group: ConstraintGroupId) -> Option<f64> {
        let node = self.group(group).map(|group_data| group_data.single_node().clone())?;
        let (li, ni) = (self.snap_layer_index(&node), self.snap_node_id(&node));
        self.barycenter_states
            .get(li).and_then(|l| l.get(ni)).and_then(|o| o.as_ref())
            .and_then(|s| s.barycenter)
    }

    fn set_group_barycenter(&mut self, group: ConstraintGroupId, barycenter: Option<f64>) {
        let nodes = match self.group(group) {
            Some(group_data) => group_data.nodes.clone(),
            None => return,
        };
        for node in nodes {
            let (li, ni) = (self.snap_layer_index(&node), self.snap_node_id(&node));
            if let Some(state) = self.barycenter_states
                .get_mut(li).and_then(|l| l.get_mut(ni)).and_then(|o| o.as_mut())
            {
                state.barycenter = barycenter;
            }
        }
    }

    fn init_node_level(&mut self, node: &LNodeRef, full_init: bool) {
        let layer_index = self.snap_layer_index(node);
        let node_index = self.snap_node_id(node);

        if self.constraint_groups.get(layer_index).is_some() {
            let group_id = self.add_group(ConstraintGroup::new(node.clone()));
            let layer_groups = &mut self.constraint_groups[layer_index];
            if node_index >= layer_groups.len() {
                layer_groups.resize(node_index + 1, None);
            }
            layer_groups[node_index] = Some(group_id);
        }

        if full_init {
            if let Some(layer_states) = self.barycenter_states.get_mut(layer_index) {
                if node_index >= layer_states.len() {
                    layer_states.resize(node_index + 1, None);
                }
                layer_states[node_index] = Some(BarycenterState::new(node.clone()));
            }

            let layout_unit = node.lock().get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT);
            if let Some(layout_unit) = layout_unit {
                let key = node_ptr_id(&layout_unit);
                self.layout_units.entry(key).or_default().push(node.clone());
            }
        }
    }
}

impl IInitializable for ForsterConstraintResolver {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
        if layer_index == 0 {
            self.layout_units.clear();
            self.constraint_group_arena.clear();
        }
        if layer_index >= self.barycenter_states.len() {
            self.barycenter_states
                .resize_with(layer_index + 1, Vec::new);
        }
        if layer_index >= self.constraint_groups.len() {
            self.constraint_groups
                .resize_with(layer_index + 1, Vec::new);
        }

        let layer_len = node_order[layer_index].len();
        self.barycenter_states[layer_index] = vec![None; layer_len];
        self.constraint_groups[layer_index] = vec![None; layer_len];

        if let Some(first_node) = node_order[layer_index].first() {
            if let Some(layer) = first_node
                .lock().layer()
            {
                {
                    let mut layer_guard = layer.lock();
                    layer_guard.graph_element().id = layer_index as i32;
                }
            }
        }
    }

    fn init_at_node_level(
        &mut self,
        layer_index: usize,
        node_index: usize,
        node_order: &[Vec<LNodeRef>],
    ) {
        if let Some(node) = node_order
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
        {
            {
                let mut node_guard = node.lock();
                node_guard.shape().graph_element().id = node_index as i32;
            }
            self.init_node_level(node, true);
        }
    }
}

#[derive(Clone)]
struct ConstraintGroup {
    summed_weight: f64,
    degree: i32,
    nodes: Vec<LNodeRef>,
    outgoing_constraints: Vec<ConstraintGroupId>,
    incoming_constraints: Vec<ConstraintGroupId>,
    incoming_constraints_count: usize,
}

impl ConstraintGroup {
    fn new(node: LNodeRef) -> Self {
        ConstraintGroup {
            summed_weight: 0.0,
            degree: 0,
            nodes: vec![node],
            outgoing_constraints: Vec::new(),
            incoming_constraints: Vec::new(),
            incoming_constraints_count: 0,
        }
    }

    fn new_with_nodes(nodes: Vec<LNodeRef>) -> Self {
        ConstraintGroup {
            summed_weight: 0.0,
            degree: 0,
            nodes,
            outgoing_constraints: Vec::new(),
            incoming_constraints: Vec::new(),
            incoming_constraints_count: 0,
        }
    }

    fn outgoing_constraints_mut(&mut self) -> &mut Vec<ConstraintGroupId> {
        &mut self.outgoing_constraints
    }

    fn reset_outgoing_constraints(&mut self) {
        self.outgoing_constraints.clear();
    }

    fn has_outgoing_constraints(&self) -> bool {
        !self.outgoing_constraints.is_empty()
    }

    fn reset_incoming_constraints(&mut self) {
        self.incoming_constraints.clear();
    }

    fn single_node(&self) -> &LNodeRef {
        &self.nodes[0]
    }
}

fn contains_group(list: &[ConstraintGroupId], target: ConstraintGroupId) -> bool {
    list.contains(&target)
}

fn remove_group(list: &mut Vec<ConstraintGroupId>, target: ConstraintGroupId) -> bool {
    if let Some(index) = list.iter().position(|candidate| *candidate == target) {
        list.remove(index);
        return true;
    }
    false
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock().shape().graph_element().id as usize
}

fn layer_index(node: &LNodeRef) -> usize {
    let layer = node.lock().layer();
    if let Some(layer) = layer {
        {
            let mut layer_guard = layer.lock();
            return layer_guard.graph_element().id as usize;
        }
    }
    0
}

fn node_ptr_id(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}

fn group_contains_pump(resolver: &ForsterConstraintResolver, group_id: ConstraintGroupId) -> bool {
    resolver.group(group_id).is_some_and(|group_data| {
        group_data.nodes.iter().any(|node| {
            node.lock().to_string().contains("pumpOutletPressure")
        })
    })
}

fn format_group_list(
    resolver: &ForsterConstraintResolver,
    groups: &[ConstraintGroupId],
) -> String {
    groups
        .iter()
        .map(|group| format_group(resolver, *group))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn format_group(resolver: &ForsterConstraintResolver, group: ConstraintGroupId) -> String {
    let bary = resolver.group_barycenter(group);
    let names = resolver
        .group(group)
        .map(|group_data| {
            group_data
                .nodes
                .iter()
                .map(|node| {
                    node.lock().to_string()
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_else(|| "<poisoned-group>".to_owned());
    format!("[{}]<{:?}>", names, bary)
}
