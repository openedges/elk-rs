use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use org_eclipse_elk_graph::org::eclipse::elk::graph::util::elk_mutex::Mutex;

use crate::org::eclipse::elk::alg::layered::graph::{LNodeRef, NodeType};
use crate::org::eclipse::elk::alg::layered::options::InternalProperties;
use crate::org::eclipse::elk::alg::layered::p3order::barycenter_heuristic::BarycenterState;
use crate::org::eclipse::elk::alg::layered::p3order::counting::IInitializable;

pub struct ForsterConstraintResolver {
    constraints_between_non_dummies: bool,
    layout_units: BTreeMap<usize, Vec<LNodeRef>>,
    barycenter_states: Vec<Vec<Option<Arc<Mutex<BarycenterState>>>>>,
    constraint_groups: Vec<Vec<Option<ConstraintGroupRef>>>,
}

type ConstraintGroupRef = Arc<Mutex<ConstraintGroup>>;

impl ForsterConstraintResolver {
    pub fn new(
        _current_node_order: &[Vec<LNodeRef>],
        constraints_between_non_dummies: bool,
    ) -> Self {
        ForsterConstraintResolver {
            constraints_between_non_dummies,
            layout_units: BTreeMap::new(),
            barycenter_states: Vec::new(),
            constraint_groups: Vec::new(),
        }
    }

    pub fn barycenter_states(&self) -> Vec<Vec<Arc<Mutex<BarycenterState>>>> {
        self.barycenter_states
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|state| state.clone().expect("barycenter state missing"))
                    .collect()
            })
            .collect()
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
        let mut groups: Vec<ConstraintGroupRef> = Vec::with_capacity(nodes.len());
        for node in nodes.iter() {
            let group = self.group_of(node);
            groups.push(group);
        }

        let trace = std::env::var_os("ELK_TRACE_FORSTER_GROUPS").is_some()
            && groups.iter().any(group_contains_pump);
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
                    format_group(self, &first),
                    format_group(self, &second)
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
        for group in groups.iter() {
            let group_barycenter = self.group_barycenter(group);
            if let Ok(group_guard) = group.lock() {
                for node in group_guard.nodes.iter() {
                    nodes.push(node.clone());
                    if let Some(state) = self.state_of(node) {
                        if let Ok(mut state_guard) = state.lock() {
                            state_guard.barycenter = group_barycenter;
                        }
                    }
                }
            }
        }
    }

    fn build_constraints_graph(
        &mut self,
        groups: &[ConstraintGroupRef],
        only_between_normal_nodes: bool,
    ) {
        let trace = std::env::var_os("ELK_TRACE_FORSTER_GROUPS").is_some()
            && groups.iter().any(group_contains_pump);

        for group in groups.iter() {
            if let Ok(mut group_guard) = group.lock() {
                group_guard.reset_outgoing_constraints();
                group_guard.incoming_constraints_count = 0;
            }
        }

        let mut last_non_dummy_node: Option<LNodeRef> = None;
        for group in groups.iter() {
            let node = match group
                .lock()
                .ok()
                .map(|group_guard| group_guard.single_node().clone())
            {
                Some(node) => node,
                None => continue,
            };
            if only_between_normal_nodes {
                let node_type = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type())
                    .unwrap_or(NodeType::Normal);
                if node_type != NodeType::Normal {
                    continue;
                }
            }

            let successors = node
                .lock()
                .ok()
                .and_then(|mut node_guard| {
                    node_guard.get_property(InternalProperties::IN_LAYER_SUCCESSOR_CONSTRAINTS)
                })
                .unwrap_or_default();
            for successor in successors {
                if only_between_normal_nodes {
                    let successor_type = successor
                        .lock()
                        .ok()
                        .map(|node_guard| node_guard.node_type())
                        .unwrap_or(NodeType::Normal);
                    if successor_type != NodeType::Normal {
                        continue;
                    }
                }
                let successor_group = self.group_of(&successor);
                if let Ok(mut group_guard) = group.lock() {
                    group_guard
                        .outgoing_constraints_mut()
                        .push(successor_group.clone());
                }
                if let Ok(mut successor_guard) = successor_group.lock() {
                    successor_guard.incoming_constraints_count += 1;
                };
            }

            if !only_between_normal_nodes {
                let node_type = node
                    .lock()
                    .ok()
                    .map(|node_guard| node_guard.node_type())
                    .unwrap_or(NodeType::Normal);
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
                                if let Ok(mut last_guard) = last_group.lock() {
                                    last_guard
                                        .outgoing_constraints_mut()
                                        .push(current_group.clone());
                                }
                                if let Ok(mut current_guard) = current_group.lock() {
                                    current_guard.incoming_constraints_count += 1;
                                };
                            }
                        }
                    }
                    last_non_dummy_node = Some(node.clone());
                }
            }
        }

        if trace {
            for group in groups {
                let (incoming_count, outgoing) = group
                    .lock()
                    .ok()
                    .map(|group_guard| {
                        let outgoing = group_guard
                            .outgoing_constraints
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|target| format_group(self, &target))
                            .collect::<Vec<_>>()
                            .join(", ");
                        (group_guard.incoming_constraints_count, outgoing)
                    })
                    .unwrap_or((0, String::new()));
                eprintln!(
                    "crossmin: forster graph group={} incoming_count={} outgoing=[{}]",
                    format_group(self, group),
                    incoming_count,
                    outgoing
                );
            }
        }
    }

    fn find_violated_constraint(
        &mut self,
        groups: &[ConstraintGroupRef],
    ) -> Option<(ConstraintGroupRef, ConstraintGroupRef)> {
        let mut active_groups: VecDeque<ConstraintGroupRef> = VecDeque::new();
        let mut index_map: BTreeMap<usize, usize> = BTreeMap::new();
        for (index, group) in groups.iter().enumerate() {
            index_map.insert(group_ptr_id(group), index);
            if let Ok(mut group_guard) = group.lock() {
                group_guard.reset_incoming_constraints();
                if group_guard.has_outgoing_constraints()
                    && group_guard.incoming_constraints_count == 0
                {
                    active_groups.push_back(group.clone());
                }
            }
        }

        while let Some(group) = active_groups.pop_front() {
            let incoming = match group.lock().ok() {
                Some(group_guard) => {
                    if group_guard.has_incoming_constraints() {
                        group_guard.incoming_constraints.clone().unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                }
                None => Vec::new(),
            };
            if !incoming.is_empty() {
                for predecessor in incoming {
                    let pred_bary = self.group_barycenter(&predecessor).unwrap_or(0.0);
                    let group_bary = self.group_barycenter(&group).unwrap_or(0.0);
                    // Java compares via .floatValue() (f32 truncation)
                    if (pred_bary as f32) == (group_bary as f32) {
                        let pred_index = index_map
                            .get(&group_ptr_id(&predecessor))
                            .copied()
                            .unwrap_or(0);
                        let group_index =
                            index_map.get(&group_ptr_id(&group)).copied().unwrap_or(0);
                        if pred_index > group_index {
                            return Some((predecessor, group));
                        }
                    } else if pred_bary > group_bary {
                        return Some((predecessor, group));
                    }
                }
            }

            let outgoing = group
                .lock()
                .ok()
                .and_then(|group_guard| group_guard.outgoing_constraints.clone())
                .unwrap_or_default();
            for successor in outgoing {
                if let Ok(mut successor_guard) = successor.lock() {
                    let incoming_count = successor_guard.incoming_constraints_count;
                    let list_len = {
                        let list = successor_guard
                            .incoming_constraints
                            .get_or_insert_with(Vec::new);
                        list.insert(0, group.clone());
                        list.len()
                    };
                    if incoming_count == list_len {
                        active_groups.push_back(successor.clone());
                    }
                }
            }
        }

        None
    }

    fn handle_violated_constraint(
        &mut self,
        first: ConstraintGroupRef,
        second: ConstraintGroupRef,
        groups: &mut Vec<ConstraintGroupRef>,
    ) {
        let new_group = self.merge_groups(&first, &second);
        let new_barycenter = self.group_barycenter(&new_group).unwrap_or(0.0);

        let mut already_inserted = false;
        let mut index = 0usize;
        while index < groups.len() {
            let group = groups[index].clone();
            if Arc::ptr_eq(&group, &first) || Arc::ptr_eq(&group, &second) {
                groups.remove(index);
                continue;
            }

            let group_barycenter = self.group_barycenter(&group).unwrap_or(0.0);
            if !already_inserted && group_barycenter > new_barycenter {
                groups.insert(index, new_group.clone());
                already_inserted = true;
                continue;
            }

            if let Ok(mut group_guard) = group.lock() {
                if group_guard.has_outgoing_constraints() {
                    let outgoing = group_guard.outgoing_constraints_mut();
                    let first_removed = remove_group(outgoing, &first);
                    let second_removed = remove_group(outgoing, &second);
                    if first_removed || second_removed {
                        outgoing.push(new_group.clone());
                        if let Ok(mut new_guard) = new_group.lock() {
                            new_guard.incoming_constraints_count += 1;
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
        group1: &ConstraintGroupRef,
        group2: &ConstraintGroupRef,
    ) -> ConstraintGroupRef {
        let (nodes1, sum1, deg1, outgoing1) = match group1.lock() {
            Ok(g1) => (
                g1.nodes.clone(),
                g1.summed_weight,
                g1.degree,
                g1.outgoing_constraints.clone(),
            ),
            Err(_) => (Vec::new(), 0.0, 0, None),
        };
        let (nodes2, sum2, deg2, outgoing2) = match group2.lock() {
            Ok(g2) => (
                g2.nodes.clone(),
                g2.summed_weight,
                g2.degree,
                g2.outgoing_constraints.clone(),
            ),
            Err(_) => (Vec::new(), 0.0, 0, None),
        };
        let mut nodes = nodes1;
        nodes.extend(nodes2);

        let mut new_group = ConstraintGroup::new_with_nodes(nodes);
        new_group.summed_weight = sum1 + sum2;
        new_group.degree = deg1 + deg2;

        if let Some(list1) = outgoing1.as_ref() {
            let mut outgoing = list1.clone();
            remove_group(&mut outgoing, group2);
            if let Some(list2) = outgoing2.as_ref() {
                for candidate in list2 {
                    if Arc::ptr_eq(candidate, group1) {
                        continue;
                    }
                    if contains_group(&outgoing, candidate) {
                        if let Ok(mut candidate_guard) = candidate.lock() {
                            candidate_guard.incoming_constraints_count =
                                candidate_guard.incoming_constraints_count.saturating_sub(1);
                        }
                    } else {
                        outgoing.push(candidate.clone());
                    }
                }
            }
            new_group.outgoing_constraints = Some(outgoing);
        } else if let Some(list2) = outgoing2.as_ref() {
            let mut outgoing = list2.clone();
            remove_group(&mut outgoing, group1);
            new_group.outgoing_constraints = Some(outgoing);
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

        let new_group_ref = Arc::new(Mutex::new(new_group));
        self.set_group_barycenter(&new_group_ref, barycenter);
        new_group_ref
    }

    fn group_of(&self, node: &LNodeRef) -> ConstraintGroupRef {
        let layer_index = layer_index(node);
        let node_index = node_id(node);
        self.constraint_groups
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
            .and_then(|group| group.clone())
            .expect("constraint group missing")
    }

    fn state_of(&self, node: &LNodeRef) -> Option<Arc<Mutex<BarycenterState>>> {
        let layer_index = layer_index(node);
        let node_index = node_id(node);
        self.barycenter_states
            .get(layer_index)
            .and_then(|layer| layer.get(node_index))
            .and_then(|state| state.clone())
    }

    fn group_barycenter(&self, group: &ConstraintGroupRef) -> Option<f64> {
        let node = match group.lock().ok() {
            Some(group_guard) => group_guard.single_node().clone(),
            None => return None,
        };
        self.state_of(&node).and_then(|state| {
            state
                .lock()
                .ok()
                .and_then(|state_guard| state_guard.barycenter)
        })
    }

    fn set_group_barycenter(&self, group: &ConstraintGroupRef, barycenter: Option<f64>) {
        let nodes = match group.lock().ok() {
            Some(group_guard) => group_guard.nodes.clone(),
            None => return,
        };
        for node in nodes {
            if let Some(state) = self.state_of(&node) {
                if let Ok(mut state_guard) = state.lock() {
                    state_guard.barycenter = barycenter;
                }
            }
        }
    }

    fn init_node_level(&mut self, node: &LNodeRef, full_init: bool) {
        let layer_index = layer_index(node);
        let node_index = node_id(node);

        if let Some(layer_groups) = self.constraint_groups.get_mut(layer_index) {
            if node_index >= layer_groups.len() {
                layer_groups.resize(node_index + 1, None);
            }
            layer_groups[node_index] =
                Some(Arc::new(Mutex::new(ConstraintGroup::new(node.clone()))));
        }

        if full_init {
            if let Some(layer_states) = self.barycenter_states.get_mut(layer_index) {
                if node_index >= layer_states.len() {
                    layer_states.resize(node_index + 1, None);
                }
                layer_states[node_index] =
                    Some(Arc::new(Mutex::new(BarycenterState::new(node.clone()))));
            }

            let layout_unit = node.lock().ok().and_then(|mut node_guard| {
                node_guard.get_property(InternalProperties::IN_LAYER_LAYOUT_UNIT)
            });
            if let Some(layout_unit) = layout_unit {
                let key = node_ptr_id(&layout_unit);
                self.layout_units.entry(key).or_default().push(node.clone());
            }
        }
    }
}

impl IInitializable for ForsterConstraintResolver {
    fn init_at_layer_level(&mut self, layer_index: usize, node_order: &[Vec<LNodeRef>]) {
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
                .lock()
                .ok()
                .and_then(|node_guard| node_guard.layer())
            {
                if let Ok(mut layer_guard) = layer.lock() {
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
            if let Ok(mut node_guard) = node.lock() {
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
    outgoing_constraints: Option<Vec<ConstraintGroupRef>>,
    incoming_constraints: Option<Vec<ConstraintGroupRef>>,
    incoming_constraints_count: usize,
}

impl ConstraintGroup {
    fn new(node: LNodeRef) -> Self {
        ConstraintGroup {
            summed_weight: 0.0,
            degree: 0,
            nodes: vec![node],
            outgoing_constraints: None,
            incoming_constraints: None,
            incoming_constraints_count: 0,
        }
    }

    fn new_with_nodes(nodes: Vec<LNodeRef>) -> Self {
        ConstraintGroup {
            summed_weight: 0.0,
            degree: 0,
            nodes,
            outgoing_constraints: None,
            incoming_constraints: None,
            incoming_constraints_count: 0,
        }
    }

    fn outgoing_constraints_mut(&mut self) -> &mut Vec<ConstraintGroupRef> {
        if self.outgoing_constraints.is_none() {
            self.outgoing_constraints = Some(Vec::new());
        }
        self.outgoing_constraints
            .as_mut()
            .expect("outgoing constraints")
    }

    fn reset_outgoing_constraints(&mut self) {
        self.outgoing_constraints = None;
    }

    fn has_outgoing_constraints(&self) -> bool {
        self.outgoing_constraints
            .as_ref()
            .map(|list| !list.is_empty())
            .unwrap_or(false)
    }

    fn reset_incoming_constraints(&mut self) {
        self.incoming_constraints = None;
    }

    fn has_incoming_constraints(&self) -> bool {
        self.incoming_constraints
            .as_ref()
            .map(|list| !list.is_empty())
            .unwrap_or(false)
    }

    fn single_node(&self) -> &LNodeRef {
        &self.nodes[0]
    }
}

fn contains_group(list: &[ConstraintGroupRef], target: &ConstraintGroupRef) -> bool {
    list.iter().any(|candidate| Arc::ptr_eq(candidate, target))
}

fn remove_group(list: &mut Vec<ConstraintGroupRef>, target: &ConstraintGroupRef) -> bool {
    if let Some(index) = list
        .iter()
        .position(|candidate| Arc::ptr_eq(candidate, target))
    {
        list.remove(index);
        return true;
    }
    false
}

fn group_ptr_id(group: &ConstraintGroupRef) -> usize {
    Arc::as_ptr(group) as usize
}

fn node_id(node: &LNodeRef) -> usize {
    node.lock()
        .ok()
        .map(|mut node_guard| node_guard.shape().graph_element().id as usize)
        .unwrap_or(0)
}

fn layer_index(node: &LNodeRef) -> usize {
    let layer = node.lock().ok().and_then(|node_guard| node_guard.layer());
    if let Some(layer) = layer {
        if let Ok(mut layer_guard) = layer.lock() {
            return layer_guard.graph_element().id as usize;
        }
    }
    0
}

fn node_ptr_id(node: &LNodeRef) -> usize {
    Arc::as_ptr(node) as usize
}

fn group_contains_pump(group: &ConstraintGroupRef) -> bool {
    group.lock().ok().is_some_and(|group_guard| {
        group_guard.nodes.iter().any(|node| {
            node.lock()
                .ok()
                .map(|mut node_guard| node_guard.to_string().contains("pumpOutletPressure"))
                .unwrap_or(false)
        })
    })
}

fn format_group_list(
    resolver: &ForsterConstraintResolver,
    groups: &[ConstraintGroupRef],
) -> String {
    groups
        .iter()
        .map(|group| format_group(resolver, group))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn format_group(resolver: &ForsterConstraintResolver, group: &ConstraintGroupRef) -> String {
    let bary = resolver.group_barycenter(group);
    let names = group
        .lock()
        .ok()
        .map(|group_guard| {
            group_guard
                .nodes
                .iter()
                .map(|node| {
                    node.lock()
                        .ok()
                        .map(|mut node_guard| node_guard.to_string())
                        .unwrap_or_else(|| "<poisoned-node>".to_owned())
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_else(|| "<poisoned-group>".to_owned());
    format!("[{}]<{:?}>", names, bary)
}
