use std::collections::HashMap;

use org_eclipse_elk_core::org::eclipse::elk::core::alg::i_layout_processor::ILayoutProcessor;
use org_eclipse_elk_core::org::eclipse::elk::core::math::kvector::KVector;
use org_eclipse_elk_core::org::eclipse::elk::core::options::direction::Direction;
use org_eclipse_elk_core::org::eclipse::elk::core::util::{IElkProgressMonitor, Pair, Triple};

use crate::org::eclipse::elk::alg::mrtree::graph::{TGraphRef, TNodeRef};
use crate::org::eclipse::elk::alg::mrtree::options::{
    EdgeRoutingMode, InternalProperties, MrTreeOptions,
};
use crate::org::eclipse::elk::alg::mrtree::tree_util::TreeUtil;

#[derive(Default)]
pub struct CompactionProcessor {
    levels: Vec<Pair<f64, f64>>,
}

impl ILayoutProcessor<TGraphRef> for CompactionProcessor {
    fn process(&mut self, graph: &mut TGraphRef, progress_monitor: &mut dyn IElkProgressMonitor) {
        progress_monitor.begin("Process compaction", 1.0);

        // Batch all graph property reads in a single lock
        let (enabled, direction, node_node_spacing, routing_mode) = {
            let mut graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            let enabled = graph_guard
                .get_property(MrTreeOptions::COMPACTION)
                .unwrap_or(false);
            let direction = graph_guard
                .get_property(MrTreeOptions::DIRECTION)
                .unwrap_or(Direction::Undefined);
            let spacing = graph_guard
                .get_property(MrTreeOptions::SPACING_NODE_NODE)
                .unwrap_or(0.0);
            let routing = graph_guard
                .get_property(MrTreeOptions::EDGE_ROUTING_MODE)
                .unwrap_or(EdgeRoutingMode::AvoidOverlap);
            (enabled, direction, spacing, routing)
        };

        if !enabled {
            progress_monitor.done();
            return;
        }

        self.set_up_levels(graph, direction);
        self.compute_node_constraints(graph, node_node_spacing / 4.0);

        let nodes = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => {
                    progress_monitor.done();
                    return;
                }
            };
            graph_guard.nodes().clone()
        };

        let mut nodes_sorted = nodes.clone();
        let dir_vec = TreeUtil::get_direction_vector(direction);
        // Pre-extract sort keys and node data — O(n) locks instead of O(n log n)
        // Also pre-extract is_root and size to avoid per-node locks in main loop
        let mut sort_keys: HashMap<usize, f64> = HashMap::with_capacity(nodes.len());
        let mut node_is_root: HashMap<usize, bool> = HashMap::with_capacity(nodes.len());
        let mut node_sizes: HashMap<usize, KVector> = HashMap::with_capacity(nodes.len());
        for n in &nodes {
            let key = node_key(n);
            if let Some(mut guard) = n.lock_ok() {
                let pos = *guard.position_ref();
                sort_keys.insert(key, dir_vec.dot_product(&pos));
                node_sizes.insert(key, *guard.size_ref());
                node_is_root.insert(
                    key,
                    guard.get_property(InternalProperties::ROOT).unwrap_or(false),
                );
            }
        }
        nodes_sorted.sort_by(|a, b| {
            let a_key = sort_keys.get(&node_key(a)).copied().unwrap_or(0.0);
            let b_key = sort_keys.get(&node_key(b)).copied().unwrap_or(0.0);
            a_key
                .partial_cmp(&b_key)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for node in nodes_sorted {
            let nk = node_key(&node);
            if node_is_root.get(&nk).copied().unwrap_or(false) {
                continue;
            }

            let (dependent, parent) = (
                self.get_lowest_dependent_node(&node, direction),
                TreeUtil::get_lowest_parent(&node, graph, direction),
            );

            let mut new_pos = 0.0;
            let mut new_pos_size = 0.0;
            let size = node_sizes.get(&nk).copied().unwrap_or_default();

            if let Some(dep) = dependent.clone() {
                // Single lock for both pos and size
                let (dep_pos, dep_size) = dep
                    .lock_ok()
                    .map(|n| (*n.position_ref(), *n.size_ref()))
                    .unwrap_or_default();
                if let Some(parent) = parent.clone() {
                    // Single lock for both pos and size
                    let (parent_pos, parent_size) = parent
                        .lock_ok()
                        .map(|n| (*n.position_ref(), *n.size_ref()))
                        .unwrap_or_default();
                    match direction {
                        Direction::Left => {
                            new_pos = dep_pos.x - node_node_spacing - size.x;
                            let alt = parent_pos.x - node_node_spacing - size.x;
                            if alt < new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.x;
                        }
                        Direction::Right => {
                            new_pos = dep_pos.x + dep_size.x + node_node_spacing;
                            let alt = parent_pos.x + parent_size.x + node_node_spacing;
                            if alt > new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.x;
                        }
                        Direction::Up => {
                            new_pos = dep_pos.y - node_node_spacing - size.y;
                            let alt = parent_pos.y - node_node_spacing - size.y;
                            if alt < new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.y;
                        }
                        Direction::Down | Direction::Undefined => {
                            new_pos = dep_pos.y + dep_size.y + node_node_spacing;
                            let alt = parent_pos.y + parent_size.y + node_node_spacing;
                            if alt > new_pos {
                                new_pos = alt;
                            }
                            new_pos_size = new_pos + size.y;
                        }
                    }
                }
            } else if let Some(parent) = parent.clone() {
                // Single lock for both pos and size
                let (parent_pos, parent_size) = parent
                    .lock_ok()
                    .map(|n| (*n.position_ref(), *n.size_ref()))
                    .unwrap_or_default();
                match direction {
                    Direction::Left => {
                        new_pos = parent_pos.x - node_node_spacing - size.x;
                        new_pos_size = new_pos + size.x;
                    }
                    Direction::Right => {
                        new_pos = parent_pos.x + parent_size.x + node_node_spacing;
                        new_pos_size = new_pos + size.x;
                    }
                    Direction::Up => {
                        new_pos = parent_pos.y - node_node_spacing - size.y;
                        new_pos_size = new_pos + size.y;
                    }
                    Direction::Down | Direction::Undefined => {
                        new_pos = parent_pos.y + parent_size.y + node_node_spacing;
                        new_pos_size = new_pos + size.y;
                    }
                }
            }

            if routing_mode == EdgeRoutingMode::AvoidOverlap {
                let level = self
                    .levels
                    .iter()
                    .position(|pair| pair.first() <= &new_pos && pair.second() >= &new_pos_size);

                let chosen_level = if let Some(index) = level {
                    Some((index, self.levels[index].clone()))
                } else if direction == Direction::Left || direction == Direction::Up {
                    self.levels
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, pair)| pair.first() <= &new_pos)
                        .map(|(idx, pair)| (idx, pair.clone()))
                } else {
                    self.levels
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, pair)| pair.first() >= &new_pos)
                        .map(|(idx, pair)| (idx, pair.clone()))
                };

                if let Some((index, level_pair)) = chosen_level {
                    if let Some(mut node_guard) = node.lock_ok() {
                        if direction.is_horizontal() {
                            node_guard.position().x = *level_pair.first();
                        } else {
                            node_guard.position().y = *level_pair.first();
                        }
                        let current_level = node_guard
                            .get_property(MrTreeOptions::TREE_LEVEL)
                            .unwrap_or(0);
                        if index > 0 && current_level != index as i32 {
                            node_guard.set_property(
                                InternalProperties::COMPACT_LEVEL_ASCENSION,
                                Some(true),
                            );
                            node_guard.set_property(MrTreeOptions::TREE_LEVEL, Some(index as i32));
                        }
                    }
                }
            } else if let Some(mut node_guard) = node.lock_ok() {
                if direction.is_horizontal() {
                    node_guard.position().x = new_pos;
                } else {
                    node_guard.position().y = new_pos;
                }
            }
        }

        progress_monitor.done();
    }
}

impl CompactionProcessor {
    fn set_up_levels(&mut self, graph: &TGraphRef, direction: Direction) {
        self.levels.clear();

        let nodes = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            graph_guard.nodes().clone()
        };

        for node in nodes {
            if let Some(mut node_guard) = node.lock_ok() {
                let level = node_guard
                    .get_property(MrTreeOptions::TREE_LEVEL)
                    .unwrap_or(0) as usize;
                while level >= self.levels.len() {
                    self.levels.push(Pair::of(f64::MAX, -f64::MAX));
                }
                let pos = node_guard.position_ref();
                let size = node_guard.size_ref();
                if direction.is_horizontal() {
                    if pos.x < *self.levels[level].first() {
                        self.levels[level].set_first(pos.x);
                    }
                    if pos.x + size.x > *self.levels[level].second() {
                        self.levels[level].set_second(pos.x + size.x);
                    }
                } else {
                    if pos.y < *self.levels[level].first() {
                        self.levels[level].set_first(pos.y);
                    }
                    if pos.y + size.y > *self.levels[level].second() {
                        self.levels[level].set_second(pos.y + size.y);
                    }
                }
            }
        }
    }

    fn compute_node_constraints(&mut self, graph: &TGraphRef, node_node_spacing: f64) {
        let direction = graph
            .lock_ok()
            .and_then(|mut g| g.get_property(MrTreeOptions::DIRECTION))
            .unwrap_or(Direction::Undefined);
        let right = if direction.is_horizontal() {
            Direction::Down
        } else {
            Direction::Right
        };

        let nodes = {
            let graph_guard = match graph.lock_ok() {
            Some(guard) => guard,
            None => return,
            };
            graph_guard.nodes().clone()
        };

        let actual_nodes: Vec<TNodeRef> = nodes
            .into_iter()
            .filter(|node| {
                node.lock_ok()
                    .and_then(|node_guard| {
                        node_guard.label().map(|label| label.contains("SUPER_ROOT"))
                    })
                    .map(|contains| !contains)
                    .unwrap_or(true)
            })
            .collect();

        // Pre-extract positions into a map for insert_sorted — single lock per node
        let mut pos_cache: HashMap<usize, KVector> = HashMap::with_capacity(actual_nodes.len());
        let mut points: Vec<Triple<TNodeRef, KVector, bool>> = Vec::with_capacity(actual_nodes.len() * 2);

        for node in &actual_nodes {
            if let Some(node_guard) = node.lock_ok() {
                let pos = *node_guard.position_ref();
                let size = *node_guard.size_ref();
                let key = node_key(node);
                pos_cache.insert(key, pos);

                let mut start_pos = pos;
                start_pos.sub_values(node_node_spacing, node_node_spacing);
                points.push(Triple::new(node.clone(), start_pos, true));

                let mut end_pos = pos;
                end_pos.add_values(
                    size.x + node_node_spacing,
                    size.y + node_node_spacing,
                );
                points.push(Triple::new(node.clone(), end_pos, false));
            }
        }

        let right_vec = TreeUtil::get_direction_vector(right);
        points.sort_by(|a, b| {
            right_vec
                .dot_product(a.second())
                .partial_cmp(&right_vec.dot_product(b.second()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let dir_vec = TreeUtil::get_direction_vector(direction);
        let mut active: Vec<TNodeRef> = Vec::new();
        let mut candidates: HashMap<usize, TNodeRef> = HashMap::new();

        for point in points {
            let node = point.first().clone();
            if *point.third() {
                insert_sorted_cached(&mut active, &node, &dir_vec, &pos_cache);
                if let Some(left) = left_neighbor(&active, &node) {
                    candidates.insert(node_key(&node), left);
                }
                if let Some(right_neighbor) = right_neighbor(&active, &node) {
                    candidates.insert(node_key(&right_neighbor), node.clone());
                }
            } else {
                if let Some(left) = left_neighbor(&active, &node) {
                    if candidates
                        .get(&node_key(&node))
                        .map(|cand| std::sync::Arc::ptr_eq(cand, &left))
                        .unwrap_or(false)
                    {
                        push_constraint(&node, &left);
                    }
                }
                if let Some(right) = right_neighbor(&active, &node) {
                    if candidates
                        .get(&node_key(&right))
                        .map(|cand| std::sync::Arc::ptr_eq(cand, &node))
                        .unwrap_or(false)
                    {
                        push_constraint(&right, &node);
                    }
                }
                active.retain(|n| !std::sync::Arc::ptr_eq(n, &node));
            }
        }
    }

    fn get_lowest_dependent_node(&self, node: &TNodeRef, direction: Direction) -> Option<TNodeRef> {
        let constraints = node.lock_ok().and_then(|mut node_guard| {
            node_guard.get_property(InternalProperties::COMPACT_CONSTRAINTS)
        });
        let constraints = constraints.unwrap_or_default();
        if constraints.is_empty() {
            return None;
        }
        if constraints.len() == 1 {
            return constraints.first().cloned();
        }

        let mut best: Option<TNodeRef> = None;
        let mut best_value = 0.0_f64;
        for candidate in constraints {
            // Single lock for both pos and size
            let (candidate_pos, candidate_size) = candidate
                .lock_ok()
                .map(|n| (*n.position_ref(), *n.size_ref()))
                .unwrap_or_default();
            let value = match direction {
                Direction::Left => candidate_pos.x,
                Direction::Right => candidate_pos.x + candidate_size.x,
                Direction::Up => candidate_pos.y,
                Direction::Down | Direction::Undefined => candidate_pos.y + candidate_size.y,
            };
            let is_better = match best {
                None => true,
                Some(_) => {
                    (direction == Direction::Left || direction == Direction::Up)
                        && value < best_value
                        || (direction == Direction::Right
                            || direction == Direction::Down
                            || direction == Direction::Undefined)
                            && value > best_value
                }
            };
            if is_better {
                best_value = value;
                best = Some(candidate);
            }
        }
        best
    }
}

fn node_key(node: &TNodeRef) -> usize {
    std::sync::Arc::as_ptr(node) as usize
}

/// Insert into sorted active list using pre-cached positions — zero locks.
fn insert_sorted_cached(
    active: &mut Vec<TNodeRef>,
    node: &TNodeRef,
    dir_vec: &KVector,
    pos_cache: &HashMap<usize, KVector>,
) {
    let node_pos = pos_cache
        .get(&node_key(node))
        .copied()
        .unwrap_or_default();
    let node_proj = dir_vec.dot_product(&node_pos);
    let index = active
        .iter()
        .position(|other| {
            let other_pos = pos_cache
                .get(&node_key(other))
                .copied()
                .unwrap_or_default();
            node_proj < dir_vec.dot_product(&other_pos)
        })
        .unwrap_or(active.len());
    active.insert(index, node.clone());
}

fn left_neighbor(active: &[TNodeRef], node: &TNodeRef) -> Option<TNodeRef> {
    let index = active
        .iter()
        .position(|n| std::sync::Arc::ptr_eq(n, node))?;
    if index == 0 {
        None
    } else {
        Some(active[index - 1].clone())
    }
}

fn right_neighbor(active: &[TNodeRef], node: &TNodeRef) -> Option<TNodeRef> {
    let index = active
        .iter()
        .position(|n| std::sync::Arc::ptr_eq(n, node))?;
    if index + 1 >= active.len() {
        None
    } else {
        Some(active[index + 1].clone())
    }
}

fn push_constraint(node: &TNodeRef, constraint: &TNodeRef) {
    if let Some(mut node_guard) = node.lock_ok() {
        let mut list = node_guard
            .get_property(InternalProperties::COMPACT_CONSTRAINTS)
            .unwrap_or_else(Vec::new);
        list.push(constraint.clone());
        node_guard.set_property(InternalProperties::COMPACT_CONSTRAINTS, Some(list));
    }
}
